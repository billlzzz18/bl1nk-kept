#[derive(Clone, Debug)]
pub struct ProcessIdGetter(pub u32);

impl ProcessIdGetter {
    pub fn pid(&self) -> Option<u32> {
        Some(self.0)
    }
}

impl<T> From<&T> for ProcessIdGetter {
    fn from(_: &T) -> Self {
        Self(0)
    }
}

#[derive(Debug)]
pub struct PtyProcessInfo {
    getter: ProcessIdGetter,
}

impl PtyProcessInfo {
    pub fn new(getter: ProcessIdGetter) -> Self {
        Self { getter }
    }

    pub fn pid(&self) -> Option<u32> {
        self.getter.pid()
    }

    pub fn pid_getter(&self) -> &ProcessIdGetter {
        &self.getter
    }
}
