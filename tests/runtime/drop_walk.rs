// A model glue step yields a chain, then completes each parent. The walker is
// the same source used with generated application glue on production routes.
pub fn model_drop_step(frame: u64) -> u64 {
    if raw_load64(frame + 16) == 0 {
        raw_store64(frame + 16, 1);
        let count = raw_load64(frame);
        if count > 1 {
            raw_store64(frame + 24, 0);
            return count - 1;
        }
    }
    0
}
pub fn probe_drop() -> u64 {
    let frame = allocate(32);
    raw_store64(frame, 1024);
    raw_store64(frame + 8, 0);
    raw_store64(frame + 16, 0);
    raw_store64(frame + 24, 0);
    drop_walk(frame);
    let peak = raw_heap_tip();
    let reused = allocate(32768);
    if raw_heap_tip() != peak { return 1; }
    release(reused);
    42
}
