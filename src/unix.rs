use crate::SpellingError;
use dotenv_codegen::dotenv;
use hunspell_sys::{
    Hunhandle,
    Hunspell_add,
    Hunspell_create,
    Hunspell_destroy,
    Hunspell_free_list,
    Hunspell_spell,
    Hunspell_suggest,
};
use regex::Regex;
use std::ptr;
use std::ffi::{CStr, CString};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

static DICTIONARY_PATH: &str = dotenv!("DICTIONARY_PATH");

#[derive(Debug)]
pub struct Checker {
    boundary: Regex,
    handle: *mut Hunhandle,
}

impl Checker {
    pub fn new() -> Self {
        let aff = build_path("index.aff");
        let dic = build_path("index.dic");

        Self {
            boundary: Regex::new(r#"[^\s]+"#).unwrap(),
            handle: unsafe {
                Hunspell_create(
                    aff.as_os_str().as_bytes().as_ptr() as *const i8,
                    dic.as_os_str().as_bytes().as_ptr() as *const i8,
                )
            }
        }
    }

    pub fn check<'a>(&'a mut self, text: &'a str) -> impl Iterator<Item = SpellingError> + 'a {
        let handle = self.handle;
        self.boundary.find_iter(text).filter_map(move |location| {
            let word = location.as_str();
            let ffi_string = CString::new(word).unwrap();

            let is_err = unsafe {
                0 == Hunspell_spell(
                    handle,
                    ffi_string.as_bytes_with_nul().as_ptr() as *const i8,
                )
            };

            if is_err {
                Some(SpellingError {
                    start: location.start(),
                    length: word.len(),
                    text: word.into(),
                })
            } else {
                None
            }
        })
    }

    // Hunspell does not play well with laziness, so we're using a vec this time.
    // This is an implementation detail which I do NOT believe should be exposded to the 
    // outside world.
    pub fn suggest(&mut self, word: &str) -> impl Iterator<Item = String> {
        let mut suggestions = Vec::new();
        let ffi_string = CString::new(word).unwrap();
        
        unsafe {
            let mut results = ptr::null_mut();
            let count = Hunspell_suggest(
                self.handle,
                &mut results,
                ffi_string.as_bytes_with_nul().as_ptr() as *const i8,
            );

            for i in 0..count {
                let suggestion = CStr::from_ptr(*results.offset(i as isize));
                suggestions.push(String::from(suggestion.to_str().unwrap()));
            }

            Hunspell_free_list(self.handle, &mut results, count as i32);
        }

        suggestions.into_iter()
    }

    pub fn ignore(&mut self, word: &str) {
        let cstr = CString::new(word).unwrap();
        unsafe { Hunspell_add(self.handle, cstr.as_bytes_with_nul().as_ptr() as *const i8) };
    }
}

impl Drop for Checker {
    fn drop(&mut self) {
        unsafe { Hunspell_destroy(self.handle) }
    }
}

fn build_path(path: impl AsRef<Path>) -> PathBuf {
    let root: &Path = DICTIONARY_PATH.as_ref();
    root.join(path)
}
