use crate::{SpellingError, null::Iter};
use std::ffi::OsStr;
use std::fmt::{self, Debug};
use std::iter;
use std::ops::Deref;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::ptr::{self, NonNull};
use std::ffi::OsString;
use winapi::{
    Class,
    Interface,
    shared::{
        winerror::{SUCCEEDED, S_FALSE},
        wtypesbase::{CLSCTX_INPROC_SERVER},
    },
    um::{
        combaseapi::{CoInitializeEx, CoCreateInstance},
        objbase::COINIT_MULTITHREADED,
        objidlbase::IEnumString,
        spellcheck::{IEnumSpellingError, ISpellingError, SpellCheckerFactory, ISpellChecker, ISpellCheckerFactory},
        unknwnbase::IUnknown,
    },
};

struct ComPtr<T>(NonNull<T>);

impl<T> ComPtr<T> {
    fn new(p: *mut T) -> ComPtr<T> where T: Interface {
        ComPtr(NonNull::new(p).unwrap())
    }
}

impl<T> Deref for ComPtr<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.0.as_ptr() }
    }
}

impl<T> Debug for ComPtr<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("ComPtr")
            .field(&format_args!("{:p}", self.0.as_ptr()))
            .finish()
    }
}

impl<T> Drop for ComPtr<T> {
    fn drop(&mut self) {
        unsafe {
            let unknown = self.0.as_ptr() as *mut IUnknown;
            (*unknown).Release();
        }
    }
}

// According to Microsoft, the caller is responsible for freeing the memory allocated for their
// null-terminated strings, here. Fine. I'll free your damn strings. I think. I hope. I have no
// idea how, on any modern system, I would ever even notice a memory leak resulting from the use
// of a spellchecker.
struct StrPtr(NonNull<u16>);

impl StrPtr {
    fn new(p: *mut u16) -> Self {
        StrPtr(NonNull::new(p).unwrap())
    }
}

impl Debug for StrPtr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("StrPtr")
            .field(&format_args!("{:p}", self.0.as_ptr()))
            .finish()
    }
}

impl Drop for StrPtr {
    fn drop(&mut self) {
        unsafe { libc::free(self.0.as_ptr() as *mut libc::c_void) }
    }
}

fn wide_string(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(iter::once(0)).collect()
}

#[derive(Debug)]
pub struct Checker {
    checker: ComPtr<ISpellChecker>,
}

impl Checker {
    pub fn new() -> Self {
        let hr = unsafe { CoInitializeEx(ptr::null_mut(), COINIT_MULTITHREADED) };
        if !SUCCEEDED(hr) {
            panic!("could not initialize COM: {:?}", hr);
        }

        let mut obj = ptr::null_mut();
        let hr = unsafe {
            CoCreateInstance(
                &SpellCheckerFactory::uuidof(),
                ptr::null_mut(),
                CLSCTX_INPROC_SERVER,
                &ISpellCheckerFactory::uuidof(),
                &mut obj,
            )
        };

        assert!(SUCCEEDED(hr), "could not create spellchecker factory instance");
        let factory = ComPtr::new(obj as *mut ISpellCheckerFactory);

        let mut checker = ptr::null_mut();
        let lang = wide_string("en-US");
        let hr = unsafe { (*factory).CreateSpellChecker(lang.as_ptr(), &mut checker) };
        assert!(SUCCEEDED(hr), "could not create spellchecker instance");
        let checker = ComPtr::new(checker);

        Checker {
            checker,
        }
    }

    pub fn check(&mut self, text: &str) -> impl Iterator<Item = SpellingError> {
        if text.is_empty() {
            return ErrorIter {
                text: vec![],
                iter: None,
            };
        }

        let text = wide_string(text);
        let mut errors = ptr::null_mut();
        let hr = unsafe { (*self.checker).ComprehensiveCheck(text.as_ptr(), &mut errors) };
        assert!(SUCCEEDED(hr));
        let errors = ComPtr::new(errors);

        ErrorIter {
            text,
            iter: Some(errors),
        }
    }

    pub fn suggest(&mut self, text: &str) -> impl Iterator<Item = String> {
        if text.is_empty() {
            return SuggestIter {
                iter: None,
            };
        }

        let text = wide_string(text);
        let mut suggestions = ptr::null_mut();

        let hr = unsafe { (*self.checker).Suggest(text.as_ptr(), &mut suggestions) };
        assert!(SUCCEEDED(hr));
        
        SuggestIter {
            iter: Some(ComPtr::new(suggestions))
        }
    }

    pub fn ignore(&mut self, word: &str) {
        if word.is_empty() {
            return;
        }

        let word = wide_string(word);
        let hr = unsafe { (*self.checker).Ignore(word.as_ptr()) };
        assert!(SUCCEEDED(hr));
    }
}

struct ErrorIter {
    text: Vec<u16>,
    iter: Option<ComPtr<IEnumSpellingError>>,
}

impl ErrorIter {
    fn next_ptr(&mut self) -> Option<ComPtr<ISpellingError>> {
        let iter = self.iter.as_ref()?;
        let mut err = ptr::null_mut();

        if unsafe { (*iter).Next(&mut err) } == S_FALSE {
            return None;
        }

        Some(ComPtr::new(err))
    }
}

impl Iterator for ErrorIter {
    type Item = SpellingError;

    fn next(&mut self) -> Option<SpellingError> {
        let error = self.next_ptr()?;

        let mut start = 0;
        let mut length = 0;

        unsafe {
            (*error).get_Length(&mut length);
            (*error).get_StartIndex(&mut start);
        }

        let start = start as usize;
        let length = length as usize;

        Some(SpellingError {
            start,
            length,
            text: String::from_utf16(&self.text[start..start + length]).unwrap(),
        })
    }
}

struct SuggestIter {
    iter: Option<ComPtr<IEnumString>>,
}

impl SuggestIter {
    fn next_ptr(&mut self) -> Option<StrPtr> {
        let iter = self.iter.as_ref()?;
        let mut result = ptr::null_mut();
        let mut len = 0;

        if unsafe { (*iter).Next(1, &mut result, &mut len) } == S_FALSE || len == 0 {
            return None;
        }

        Some(StrPtr::new(result))
    }
}

impl Iterator for SuggestIter {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        let suggestion = self.next_ptr()?;
        let suggestion: Vec<u16> = Iter::new(suggestion.0.as_ptr()).cloned().collect();
        
        // FIXME: This is not the best way to do this...
        OsString::from_wide(&suggestion).into_string().ok()
    }
}
