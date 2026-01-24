use rocksdb::{DB, Options};
use std::fs::File;
use std::io::{Write, BufWriter};

fn main() {
    let path = "./data/node_9000";
    let output_path = "db_inspect_report.txt";

    let mut opts = Options::default();
    let db = DB::open_for_read_only(&opts, path, false).expect("DB Open Failed");
    let iter = db.iterator(rocksdb::IteratorMode::Start);

    let file = File::create(output_path).expect("파일 생성 실패");
    let mut writer = BufWriter::new(file);

    writeln!(writer, "\n{:=^60}", " BLOCKCHAIN STORAGE INSPECTOR (UPDATED) ").unwrap();
    println!("분석 중... 결과는 '{}'에 저장됩니다.", output_path);

    let mut count = 0;

    for item in iter {
        let (key, value) = item.expect("Iterator error");
        if key.is_empty() { continue; }

        let prefix = key[0];
        count += 1;

        match prefix {
            // 1. 블록 데이터 (Prefix 'b')
            b'b' => {
                let hash = &key[1..];
                writeln!(writer, "[BLOCK] Hash: 0x{}", hex_encode(hash)).unwrap();
                writeln!(writer, "        Size: {} bytes", value.len()).unwrap();
            }

            // 2. 블록 인덱스 (Prefix 'i')
            b'i' => {
                let height = u64::from_be_bytes(key[1..9].try_into().unwrap_or([0;8]));
                writeln!(writer, "[INDEX] Height: {} -> Block Hash: 0x{}", height, hex_encode(&value)).unwrap();
            }

            // 3. 계정 정보 (Prefix 'a')
            b'a' => {
                let addr = &key[1..];
                writeln!(writer, "[ACCOUNT] Addr: 0x{}", hex_encode(addr)).unwrap();
                writeln!(writer, "          Data(Postcard): {}", hex_encode(&value)).unwrap();
            }

            // 4. 토큰 메타데이터 (Prefix 't')
            b't' => {
                let ticker = String::from_utf8_lossy(&key[1..]);
                writeln!(writer, "[TOKEN] Ticker: {}", ticker).unwrap();
                writeln!(writer, "        Metadata(Hex): {}", hex_encode(&value)).unwrap();
            }

            // 5. 트랜잭션 포인터 (Prefix 'p') - 새로 추가된 부분!
            b'p' => {
                let tx_hash = &key[1..];
                writeln!(writer, "[TX_POINTER] Hash: 0x{}", hex_encode(tx_hash)).unwrap();
                // TransactionForDB 구조체가 postcard로 들어있으므로 크기 확인 가능
                writeln!(writer, "             Receipt Size: {} bytes", value.len()).unwrap();
            }

            // 6. 글로벌 상태 (Prefix 'g')
            b'g' => {
                if key.len() == 1 {
                    writeln!(writer, "[GLOBAL_STATE] Type: LATEST").unwrap();
                } else {
                    let height = u64::from_be_bytes(key[1..9].try_into().unwrap_or([0;8]));
                    writeln!(writer, "[GLOBAL_STATE] Type: SNAPSHOT | Height: {}", height).unwrap();
                }
                writeln!(writer, "               Size: {} bytes", value.len()).unwrap();
            }

            // 7. 스테이커 정보 (Prefix 's') - 새로 추가된 부분!
            b's' => {
                let addr = &key[1..];
                let amount = u64::from_be_bytes(value[..8].try_into().unwrap_or([0;8]));
                writeln!(writer, "[STAKER] Addr: 0x{}", hex_encode(addr)).unwrap();
                writeln!(writer, "         Amount: {} GOV", amount).unwrap();
            }

            // 8. 특수 키 (last_block, latest_snapshot_height 등)
            _ => {
                let key_name = String::from_utf8_lossy(&key);
                writeln!(writer, "[SYSTEM] Key: {} | Value: {}", key_name, hex_encode(&value)).unwrap();
            }
        }
        writeln!(writer, "{:-^60}", "").unwrap();
    }

    writer.flush().unwrap();
    println!("분석 완료! 총 {}개의 아이템을 파일에 기록했습니다.", count);
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}