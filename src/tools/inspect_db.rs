use rocksdb::{DB, Options};
use std::fs::File;
use std::io::{Write, BufWriter};

fn main() {
    let path = "./data/node_9000";
    let output_path = "db_inspect_report.txt";

    // 1. DB 열기
    let mut opts = Options::default();
    let db = DB::open_for_read_only(&opts, path, false).expect("DB Open Failed");
    let iter = db.iterator(rocksdb::IteratorMode::Start);

    // 2. 출력 파일 생성 (버퍼를 써서 성능 최적화)
    let file = File::create(output_path).expect("파일 생성 실패");
    let mut writer = BufWriter::new(file);

    writeln!(writer, "\n{:=^60}", " RAW BLOCKCHAIN DB INSPECTOR ").unwrap();
    println!("분석 중... 결과는 '{}'에 저장됩니다.", output_path);

    let mut count = 0;

    for item in iter {
        let (key, value) = item.expect("Iterator error");
        if key.is_empty() { continue; }

        let prefix = key[0];
        count += 1;

        match prefix {
            // 1. 계정 정보 (Prefix 'a')
            b'a' => {
                let addr = &key[1..];
                writeln!(writer, "[ACCOUNT] Addr: 0x{}", hex_encode(addr)).unwrap();
                writeln!(writer, "          Raw Value (Hex): {}", hex_encode(&value)).unwrap();
            }
            
            // 2. 블록 데이터 (Prefix 'b')
            b'b' => {
                let hash = &key[1..];
                writeln!(writer, "[BLOCK]   Hash: 0x{}", hex_encode(hash)).unwrap();
                writeln!(writer, "          Data Dump: {}", hex_encode(&value)).unwrap();
            }

            // 3. 인덱스 (Prefix 'i')
            b'i' => {
                if key.len() >= 9 {
                    let mut h_bytes = [0u8; 8];
                    h_bytes.copy_from_slice(&key[1..9]);
                    let height = u64::from_be_bytes(h_bytes);
                    writeln!(writer, "[INDEX]   Height: {} -> Block Hash: 0x{}", height, hex_encode(&value)).unwrap();
                }
            }

            // 4. 글로벌 상태 스냅샷 (Prefix 'g')
            b'g' => {
                if key.len() == 1 {
                    writeln!(writer, "[GS]      Type: LATEST_STATE (Key: 'g')").unwrap();
                } else if key.len() == 9 {
                    let mut h_bytes = [0u8; 8];
                    h_bytes.copy_from_slice(&key[1..9]);
                    let height = u64::from_be_bytes(h_bytes);
                    writeln!(writer, "[GS]      Type: SNAPSHOT | Height: {}", height).unwrap();
                }
                writeln!(writer, "          Value Size: {} bytes", value.len()).unwrap();
                writeln!(writer, "          Value Hex: {}", hex_encode(&value)).unwrap();
            }

            _ => {
                let key_name = String::from_utf8_lossy(&key);
                writeln!(writer, "[UNKNOWN] Key: {} | Value: {}", key_name, hex_encode(&value)).unwrap();
            }
        }
        writeln!(writer, "{:-^60}", "").unwrap();
    }

    // 버퍼 강제 출력
    writer.flush().unwrap();
    println!("분석 완료! 총 {}개의 아이템을 파일에 기록했습니다.", count);
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}