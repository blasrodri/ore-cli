use std::{
    sync::{mpsc::channel, Arc},
    time::Instant,
};
use threadpool::ThreadPool;

use drillx::equix;
use solana_rpc_client::spinner;

use crate::{args::BenchmarkArgs, Miner};

const TEST_DURATION: i64 = 30;

impl Miner {
    pub async fn benchmark(&self, args: BenchmarkArgs) {
        // Check num threads
        self.check_num_cores(args.cores);

        // Dispatch job to each thread
        let challenge = [0; 32];
        let progress_bar = Arc::new(spinner::new_progress_bar());
        progress_bar.set_message(format!(
            "Benchmarking. This will take {} sec...",
            TEST_DURATION
        ));
        let core_ids = core_affinity::get_core_ids().unwrap();
        let num_cores = core_ids.len();
        let pool = ThreadPool::new(core_ids.len());
        let (tx, rx) = channel();

        for i in core_ids {
            pool.execute({
                let tx = tx.clone();
                move || {
                    let timer = Instant::now();
                    let first_nonce = u64::MAX
                        .saturating_div(args.cores)
                        .saturating_mul(i.id as u64);
                    let mut nonce = first_nonce;
                    let mut memory = equix::SolverMemory::new();
                    loop {
                        // Return if core should not be used
                        // if (i.id as u64).ge(&args.cores) {
                        //     tx.send(0).unwrap();
                        // }

                        // // Pin to core
                        // let _ = core_affinity::set_for_current(i);

                        // Create hash
                        let _hx =
                            drillx::hash_with_memory(&mut memory, &challenge, &nonce.to_le_bytes());

                        // Increment nonce
                        nonce += 1;

                        // Exit if time has elapsed
                        if (timer.elapsed().as_secs() as i64).ge(&TEST_DURATION) {
                            break;
                        }
                    }

                    // Return hash count
                    tx.send(nonce - first_nonce).unwrap();
                }
            })
        }

        // Join handles and return best nonce
        let mut total_nonces = 0;
        let mut total_recv = 0;

        while let Ok(count) = rx.recv() {
            total_nonces += count;
            total_recv += 1;
            if total_recv == num_cores {
                break;
            }
        }

        // Update log
        progress_bar.finish_with_message(format!(
            "Hashpower: {} H/sec",
            total_nonces.saturating_div(TEST_DURATION as u64),
        ));
    }
}
