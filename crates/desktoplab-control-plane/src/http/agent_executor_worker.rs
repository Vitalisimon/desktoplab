use crate::LocalApiRouter;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub(super) fn spawn_executor_lane(
    router: Arc<Mutex<LocalApiRouter>>,
    stop: Arc<AtomicBool>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        while !stop.load(Ordering::SeqCst) {
            let claimed = router
                .lock()
                .expect("local api router lock should not be poisoned")
                .claim_next_approved_agent_action();
            let Some(claimed) = claimed else {
                thread::sleep(Duration::from_millis(20));
                continue;
            };
            let completed = claimed.execute();
            let mut router = router
                .lock()
                .expect("local api router lock should not be poisoned");
            router.complete_claimed_agent_action_deferred(completed);
            router.persist_event_outbox();
        }
    })
}
