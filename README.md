# Rust로 구현한 블록체인 노드

Rust로 P2P 블록체인 노드를 처음부터 직접 구현한 시스템 프로그래밍 학습 프로젝트입니다.

수십 개의 비동기 태스크 간 공유 상태 관리, 저지연 바이너리 직렬화, 크래시 안전한 영속 저장, 계층형 메모리 아키텍처 — AI 서비스 백엔드가 요구하는 문제들과 본질적으로 같습니다. Rust의 소유권 모델이 메모리 안전성과 동시성 정확성을 컴파일 타임에 강제한다는 점에서, 블록체인 노드를 만들면서 AI 추론 서버나 분산 학습 파이프라인에도 그대로 적용되는 시스템 설계 원칙을 익히는 것이 목표였습니다.

---

## 기술적 하이라이트

### 1. `Arc<RwLock<T>>`를 활용한 동시성 상태 관리

피어 연결, 트랜잭션 대기열, 잔액 캐시, 체인 상태 전체가 수십 개의 독립적인 비동기 태스크 사이에서 공유됩니다. 처음부터 일관된 소유권 모델을 설계해야 했습니다.

- `Arc<RwLock<Node>>` — 여러 태스크가 읽기 잠금을 동시에 획득할 수 있고, 쓰기는 배타적 접근을 보장
- `OnceLock<[u8; 32]>` — 개인 키를 시작 시 단 한 번 초기화하고 이후 어느 스레드에서도 안전하게 읽기
- `LazyLock` — 제네시스 블록, 토큰 공급량 등 전역 상수를 최초 접근 시 단 한 번 계산

Rust에서 가장 어려운 주제 중 하나인 "비동기 경계를 넘는 가변 상태 공유"를 `unsafe` 없이 직접 다루는 경험을 쌓았습니다.

```rust
pub struct NodeManage {
    pub state: Arc<RwLock<Node>>,
}
```

### 2. Tokio를 활용한 비동기 프로그래밍

노드는 서로 블로킹하지 않는 여러 독립 백그라운드 태스크를 동시에 실행합니다.

| 태스크 | 역할 |
|---|---|
| `start_miner` | 주기마다 새 블록을 생성하고 브로드캐스트 |
| `start_heartbeat` | 10초 간격으로 모든 피어에 Ping 전송 |
| `start_reconnector` | 15초 간격으로 알고 있지만 연결이 끊긴 피어에 재연결 시도 |
| `recent_message_collecter` | 5초 간격으로 만료된 메시지 중복 방지 캐시 GC |
| `start_rpc_server` | HTTP/JSON-RPC 요청을 비동기로 처리 |
| `boot` | 시작 시 초기 피어 디스커버리 |
| `handle_peer` (연결당 1개) | TCP 연결마다 독립 read 루프 |

각 태스크는 `tokio::spawn`으로 스폰되어 `Arc`를 통해 상태를 공유합니다. 메인 루프는 `TcpListener::accept`를 블로킹 없이 실행합니다.

### 3. 원시 TCP 위의 커스텀 바이너리 프로토콜

기존 프로토콜을 사용하지 않고 P2P 통신 레이어를 원시 TCP 위에 직접 설계하고 구현했습니다.

**메시지 프레이밍:** 모든 메시지는 4바이트 리틀엔디언 길이 접두사 + 본문으로 구성됩니다. 수신측은 바이트를 버퍼에 누적하며 블로킹 없이 완전한 메시지를 추출합니다.

```rust
pub fn decode_with_bytes(src: &[u8]) -> Option<(Self, usize)> {
    if src.len() < 4 { return None; }
    let message_len = u32::from_le_bytes(src[..4].try_into().ok()?) as usize;
    if message_len > 10 * 1024 * 1024 { return None; } // 10MB 하드 캡
    if src.len() < 4 + message_len { return None; }
    // ...
}
```

**읽기/쓰기 분리:** TCP 연결을 독립된 읽기/쓰기 반쪽으로 분리(`into_split()`)해, 피어마다 독립적인 read 루프를 운용하면서 쓰기는 `OwnedWriteHalf` 참조를 가진 어느 태스크에서도 dispatch 가능하게 했습니다.

**직렬화:** P2P 와이어 인코딩에는 `postcard`(결정론적 바이너리 포맷)를 사용했습니다. JSON보다 크기가 작고 바이트 레이아웃이 결정론적이어서 해시 기반 메시지 중복 방지에 필요한 조건을 만족합니다.

### 4. 가십 프로토콜 & 메시지 중복 방지

트랜잭션과 블록은 가십(epidemic) 프로토콜로 전파됩니다. 각 노드는 수신한 메시지를 발신자를 제외한 모든 연결 피어에게 릴레이합니다.

무한 루프 방지를 위해 메시지마다 Keccak256 해시를 계산해 `HashMap<Vec<u8>, Instant>`에 기록합니다. GC 태스크가 30초 이상 지난 엔트리를 주기적으로 제거합니다.

```rust
async fn mark_seen(&self, msg: &NetworkMessage) -> bool {
    let msg_id = msg.get_id(); // 직렬화된 본문의 Keccak256
    let mut state = self.state.write().await;
    if state.recent_seen_message.contains_key(&msg_id) { return false; }
    state.recent_seen_message.insert(msg_id, Instant::now());
    true
}
```

### 5. 암호학: secp256k1 ECDSA & Keccak256

트랜잭션과 블록 서명에 Ethereum의 암호화 체계를 직접 구현했습니다.

- **서명:** `k256` 크레이트(secp256k1 곡선)로 `sign_prehash` — `(r, s, v)` 형태의 65바이트 복구 가능 서명 생성
- **검증:** 서명과 메시지 해시로 공개 키를 복구하고, Ethereum 주소를 파생해 선언된 발신자와 비교 — 공개 키를 어디에도 저장하지 않음
- **주소 파생:** 비압축 공개 키 바이트 → Keccak256 → 마지막 20바이트 (Ethereum과 동일한 방식)

```rust
fn public_key_to_address(verifying_key: &VerifyingKey) -> [u8; 20] {
    let encoded = verifying_key.to_encoded_point(false); // 비압축
    let hash = Keccak256::digest(&encoded.as_bytes()[1..]); // 0x04 접두사 제거
    hash[12..].try_into().unwrap()
}
```

블록도 검증자의 주소를 블록 해시로부터 복구해 `header.validator`와 비교함으로써 서명을 검증합니다.

### 6. Merkle Tree & Merkle Patricia Trie (MPT)

**트랜잭션 Merkle root:** Keccak256 쌍별 해싱을 반복하는 방식으로 직접 구현했습니다. 홀수 길이일 경우 마지막 항목을 복제하고, 최종 32바이트 루트를 블록 헤더에 커밋합니다.

**상태 루트 (MPT):** 계정 상태는 Ethereum 호환 Merkle Patricia Trie(`eth_trie`)에 저장됩니다. 계정 상태를 RLP로 인코딩해 주소 키로 삽입하고, 트리 루트를 블록 헤더에 커밋해 전역 상태를 위변조 불가하게 만듭니다.

**중첩 MPT:** 비주요 토큰은 계정별 서브 트라이(`asset_root`)에 별도로 저장됩니다. 메인 트라이에는 간결한 계정 요약만 두고, 토큰 자산은 독립적으로 Merkle 증명이 가능하도록 이중 구조로 설계했습니다.

### 7. 상태 직렬화를 위한 RLP 인코딩

`AccountState`와 `PrimaryAsset` 타입에 Ethereum의 Recursive Length Prefix(RLP) 인코딩을 직접 구현했습니다.

```rust
impl Encodable for PrimaryAsset {
    fn rlp_append(&self, s: &mut rlp::RlpStream) {
        s.begin_list(2);
        s.append(&self.ticker);
        let buf = self.amount.to_big_endian();
        let start = buf.iter().position(|&x| x != 0).unwrap_or(31); // 선행 0 제거
        s.append(&&buf[start..]);
    }
}
```

### 8. 저수준 메모리 관리

- **고정 크기 배열:** `type Address = [u8; 20]`, `type Hash = [u8; 32]` — 힙 할당 없이 스택에서 복사 가능
- **수동 버퍼 관리:** TCP read 루프에서 `Vec<u8>`에 바이트를 누적하고 `drain(..n)`으로 처리된 바이트를 제자리에서 소비해 재할당 최소화
- **메모리 내 잔액 캐시 + eviction:** 최근 N 블록 내 활동이 없는 계정은 `retain()`으로 제거해 RAM 사용량 제한
- **원자적 RocksDB 쓰기:** 블록당 모든 상태 변경을 `WriteBatch`에 누적한 뒤 한 번에 커밋 — 크래시 시 부분 상태 오염 방지

```rust
pub fn remove_from_memory(&mut self, cur_height: u64, retention: u64) {
    self.balances.retain(|_, acc| {
        cur_height.saturating_sub(acc.last_seen_block) < retention
    });
}
```

### 9. 2계층 스토리지 아키텍처

AI 추론 시스템의 메모리 계층(GPU VRAM → 호스트 RAM → NVMe)과 동일한 사고 방식을 적용했습니다.

1. **핫 레이어 (RAM):** `GlobalBalance`가 최근 활성 계정을 `HashMap`으로 보관. 읽기/쓰기 지연 나노초 수준, retention 정책으로 eviction
2. **콜드 레이어 (RocksDB):** 타입별 키 접두사(`b'a'` 계정, `b'b'` 블록 등)를 사용하는 영속 키-값 저장소. RAM 미스 시 폴백

### 10. Ethereum JSON-RPC 호환 HTTP API

`axum`으로 구축한 HTTP 서버가 커스텀 REST 엔드포인트와 Ethereum 호환 JSON-RPC 인터페이스를 제공합니다.

- `eth_chainId`, `eth_blockNumber`, `eth_getBalance`, `eth_getBlockByNumber`
- 커스텀 엔드포인트: `/transaction`, `/nonce/{address}`, `/dashboard/state`
- 멤풀 수락 전 트랜잭션 서명 검증

### 11. 옵코드 기반 커스텀 트랜잭션 VM

```
OP_SYSTEM_REGISTER_TOKEN  0x00  — 새 토큰 생성
OP_TOKEN_MINT             0x01  — 주소에 토큰 발행
OP_TOKEN_TRANSFER         0x02  — 가스 수수료 포함 토큰 전송
OP_TOKEN_BURN             0x03  — 토큰 소각
OP_CONFIG                 0xFF  — 거버넌스 파라미터 변경
```

---

## 아키텍처 개요

```
┌─────────────────────────────────────────────────────────────┐
│                        NodeManage                           │
│                    Arc<RwLock<Node>>                         │
├──────────────┬──────────────┬──────────────┬────────────────┤
│   Network    │    Exec VM   │    Storage   │   State / MPT  │
│  (TCP P2P)   │  (Opcodes)   │  (RocksDB)   │  (eth_trie)    │
│  Gossip      │  Handlers    │  WriteBatch  │  RLP Encoding  │
│  Discovery   │  Gas / Fees  │  Prefix Keys │  Sub-Tries     │
├──────────────┴──────────────┴──────────────┴────────────────┤
│                     Crypto Layer                            │
│          secp256k1 ECDSA  ·  Keccak256  ·  Merkle Tree      │
├─────────────────────────────────────────────────────────────┤
│                   HTTP/RPC (axum)                           │
│              JSON-RPC  ·  REST  ·  MetaMask 호환             │
└─────────────────────────────────────────────────────────────┘
```

---

## 주요 의존성

| 크레이트 | 용도 |
|---|---|
| `tokio` | 비동기 런타임, TCP, 타이머 |
| `axum` | 비동기 HTTP 서버 |
| `k256` | secp256k1 ECDSA |
| `sha3` | Keccak256 해싱 |
| `rocksdb` | 임베디드 영속 키-값 저장소 |
| `eth_trie` | Ethereum 호환 Merkle Patricia Trie |
| `postcard` | P2P 와이어 포맷용 바이너리 직렬화 |
| `rlp` | Recursive Length Prefix 인코딩 |
| `serde` | 직렬화 |

---

## 로컬 실행

```bash
# 노드 1 (제네시스)
cargo run --bin node -- --config config.yaml

# 노드 2, 3
cargo run --bin node -- --config config_node2.yaml
cargo run --bin node -- --config config_node3.yaml

# RocksDB 상태 검사
cargo run --bin db-inspector
```

---

## 이 프로젝트를 통해 익힌 것

AI 서비스 백엔드 개발에 직접 연결되는 시스템 설계 원칙들을 체득했습니다.

- **비동기 공유 상태 관리** — 데드락·데이터 레이스 없이 수십 개의 비동기 태스크 간 가변 상태를 구조화하는 방법. FastAPI + asyncio 기반 AI 파이프라인 설계와 동일한 사고 방식
- **계층형 메모리 아키텍처** — RAM 캐시 + 디스크 폴백 + 명시적 eviction 정책. GPU VRAM → RAM → NVMe 계층을 갖는 ML 추론 서버와 같은 원칙
- **원자적 배치 쓰기** — 프로세스 장애 시 부분 상태 오염을 방지하는 방법. AI 파이프라인의 중간 결과 영속화에도 동일하게 적용
- **저지연 직렬화 설계** — 바이너리 프레이밍, 포맷 선택, 버퍼 관리가 처리량과 지연에 미치는 영향
- **해시 기반 중복 방지** — 가십 루프를 막는 가장 단순하고 정확한 방법. 분산 메시지 큐에서 동일 이벤트가 여러 워커에게 중복 처리되는 문제와 같은 구조
