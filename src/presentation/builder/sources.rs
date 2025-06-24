use crate::{markdown::elements::SourcePosition, presentation::builder::error::FileSourcePosition};
use std::{cell::RefCell, path::PathBuf, rc::Rc};

#[derive(Default)]
struct Inner {
    include_paths: Vec<PathBuf>,
}

#[derive(Default)]
pub(crate) struct MarkdownSources {
    inner: Rc<RefCell<Inner>>,
}

impl MarkdownSources {
    pub(crate) fn enter<P: Into<PathBuf>>(&self, path: P) -> Result<SourceGuard, MarkdownSourceError> {
        let path = path.into();
        if path.parent().is_none() {
            return Err(MarkdownSourceError::NoParent);
        }

        let mut inner = self.inner.borrow_mut();
        if inner.include_paths.contains(&path) {
            return Err(MarkdownSourceError::IncludeCycle(path));
        }
        inner.include_paths.push(path);
        Ok(SourceGuard(self.inner.clone()))
    }

    pub(crate) fn current_base_path(&self) -> PathBuf {
        self.inner
            .borrow()
            .include_paths
            .last()
            // SAFETY: we validate we know the parent before pushing into `include_paths`
            .map(|path| path.parent().expect("no parent").to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."))
    }

    pub(crate) fn resolve_source_position(&self, source_position: SourcePosition) -> FileSourcePosition {
        let file = self.inner.borrow().include_paths.last().cloned().unwrap_or_else(|| PathBuf::from("."));
        FileSourcePosition { source_position, file }
    }
}

#[must_use]
pub(crate) struct SourceGuard(Rc<RefCell<Inner>>);

impl Drop for SourceGuard {
    fn drop(&mut self) {
        self.0.borrow_mut().include_paths.pop();
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum MarkdownSourceError {
    #[error("cannot detect path's parent")]
    NoParent,

    #[error("{0:?} was already imported")]
    IncludeCycle(PathBuf),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn paths() {
        let sources = MarkdownSources::default();
        assert_eq!(sources.current_base_path(), Path::new("."));

        {
            let _guard1 = sources.enter("foo.md");
            assert_eq!(sources.current_base_path(), Path::new(""));

            {
                let _guard2 = sources.enter("inner/bar.md");
                assert_eq!(sources.current_base_path(), Path::new("inner"));
            }

            assert_eq!(sources.current_base_path(), Path::new(""));
        }
    }
}
