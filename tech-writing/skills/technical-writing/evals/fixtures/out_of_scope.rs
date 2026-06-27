// This function validates the configuration thoroughly to make absolutely sure
// that every single field is present and correctly formatted before we go ahead
// and proceed, because it is very important that we never continue running with
// an invalid configuration of any kind.
fn validate(cfg: &Config) -> Result<(), Error> {
    cfg.check_required()?;
    cfg.check_formats()
}

// (task: a new helper added in this change, currently undocumented)
fn clamp_unit(x: f32) -> f32 {
    x.clamp(0.0, 1.0)
}
