
pub trait ErrorExt<T, E, OE> {
    fn map_log_error(self, context: &str, suberror: E) -> Result<T, E>;
}

impl <T, E, OE> ErrorExt<T, E, OE> for Result<T, OE>
where OE: std::fmt::Debug {
    fn map_log_error(self, context: &str, suberror: E) -> Result<T, E> {
        self.map_err(|e| {
            log::error!("{:} : {:?}", context, e);
            suberror
        })
    }
}
