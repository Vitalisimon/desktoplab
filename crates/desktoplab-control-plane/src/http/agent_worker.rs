use crate::LocalApiRouter;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub(super) struct AgentWorker {
    handles: Vec<JoinHandle<()>>,
}

impl AgentWorker {
    pub(super) fn spawn(router: Arc<Mutex<LocalApiRouter>>, stop: Arc<AtomicBool>) -> Self {
        let executor_handle = spawn_executor_lane(router.clone(), stop.clone());
        let model_handle = spawn_model_lane(router, stop);
        Self {
            handles: vec![executor_handle, model_handle],
        }
    }
}

fn spawn_executor_lane(
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

fn spawn_model_lane(router: Arc<Mutex<LocalApiRouter>>, stop: Arc<AtomicBool>) -> JoinHandle<()> {
    thread::spawn(move || {
        while !stop.load(Ordering::SeqCst) {
            let model_turn = router
                .lock()
                .expect("local api router lock should not be poisoned")
                .claim_next_agent_model_turn();
            let Some(model_turn) = model_turn else {
                thread::sleep(Duration::from_millis(20));
                continue;
            };
            let session_id = model_turn.session_id().to_string();
            let workspace_id = model_turn.workspace_id().to_string();
            let backend_id = model_turn.backend_id().to_string();
            let progress_router = router.clone();
            let completed = model_turn.execute(|delta| {
                progress_router
                    .lock()
                    .expect("local api router lock should not be poisoned")
                    .record_agent_model_delta(&workspace_id, &session_id, &backend_id, delta);
            });
            let mut router = router
                .lock()
                .expect("local api router lock should not be poisoned");
            router.complete_agent_model_turn(completed);
            router.persist_event_outbox();
        }
    })
}

impl Drop for AgentWorker {
    fn drop(&mut self) {
        for handle in self.handles.drain(..) {
            let _ = handle.join();
        }
    }
}
