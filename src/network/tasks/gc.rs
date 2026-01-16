use std::sync::Arc;
use std::time::{Duration, Instant};
use crate::network::node::NodeManage;

impl NodeManage{
    pub async fn recent_message_collecter(self: Arc<Self>){
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        loop {
            interval.tick().await;
            let now = Instant::now();
            let mut state = self.state.write().await;

            let before_count = state.recent_seen_message.len();
            state.recent_seen_message.retain(|_, &mut time|{
                now.duration_since(time) < Duration::from_secs(30)
            });
            let after_count = state.recent_seen_message.len();
            if before_count != after_count{
                println!("GC colelcted expired message: {}", before_count - after_count);
            }
        }
    }
}