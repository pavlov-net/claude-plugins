/// A poll, not a stream: the value is a snapshot in time, captured the very
/// instant you ask for it, and it quietly drifts away the moment your attention
/// turns elsewhere, so to truly follow it you must keep on asking, again and
/// again, frame after frame.
pub fn read_headroom(&self) -> Option<f32> {
    self.current_headroom()
}
