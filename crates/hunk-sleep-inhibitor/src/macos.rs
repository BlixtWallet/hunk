use core_foundation::base::TCFType;
use core_foundation::string::CFString;
use std::ffi::c_void;
use tracing::warn;

type CFStringRef = *const c_void;
type IOPMAssertionID = u32;
type IOPMAssertionLevel = u32;
type IOReturn = i32;

const ASSERTION_REASON: &str = "Hunk is running an AI turn";
const ASSERTION_TYPE_PREVENT_USER_IDLE_SYSTEM_SLEEP: &str = "PreventUserIdleSystemSleep";
const IO_PM_ASSERTION_LEVEL_ON: IOPMAssertionLevel = 255;
const IO_RETURN_SUCCESS: IOReturn = 0;

#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOPMAssertionCreateWithName(
        assertion_type: CFStringRef,
        assertion_level: IOPMAssertionLevel,
        assertion_name: CFStringRef,
        assertion_id: *mut IOPMAssertionID,
    ) -> IOReturn;

    fn IOPMAssertionRelease(assertion_id: IOPMAssertionID) -> IOReturn;
}

#[derive(Debug, Default)]
pub(crate) struct SleepInhibitor {
    assertion: Option<MacSleepAssertion>,
}

impl SleepInhibitor {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn acquire(&mut self) {
        if self.assertion.is_some() {
            return;
        }

        match MacSleepAssertion::create(ASSERTION_REASON) {
            Ok(assertion) => self.assertion = Some(assertion),
            Err(error) => warn!(
                iokit_error = error,
                "Failed to create macOS sleep-prevention assertion"
            ),
        }
    }

    pub(crate) fn release(&mut self) {
        self.assertion = None;
    }
}

#[derive(Debug)]
struct MacSleepAssertion {
    id: IOPMAssertionID,
}

impl MacSleepAssertion {
    fn create(name: &str) -> Result<Self, IOReturn> {
        let assertion_type = CFString::new(ASSERTION_TYPE_PREVENT_USER_IDLE_SYSTEM_SLEEP);
        let assertion_name = CFString::new(name);
        let mut id: IOPMAssertionID = 0;
        let result = unsafe {
            IOPMAssertionCreateWithName(
                assertion_type.as_concrete_TypeRef().cast(),
                IO_PM_ASSERTION_LEVEL_ON,
                assertion_name.as_concrete_TypeRef().cast(),
                &mut id,
            )
        };
        if result == IO_RETURN_SUCCESS {
            Ok(Self { id })
        } else {
            Err(result)
        }
    }
}

impl Drop for MacSleepAssertion {
    fn drop(&mut self) {
        let result = unsafe { IOPMAssertionRelease(self.id) };
        if result != IO_RETURN_SUCCESS {
            warn!(
                iokit_error = result,
                "Failed to release macOS sleep-prevention assertion"
            );
        }
    }
}
