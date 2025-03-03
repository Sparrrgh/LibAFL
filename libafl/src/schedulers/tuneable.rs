//! The queue corpus scheduler implements an AFL-like queue mechanism
//! The [`TuneableScheduler`] extends the queue scheduler with a method to
//! chose the next corpus entry manually

use alloc::borrow::ToOwned;
use core::marker::PhantomData;

use serde::{Deserialize, Serialize};

use super::RemovableScheduler;
use crate::{
    corpus::{Corpus, CorpusId},
    impl_serdeany,
    inputs::UsesInput,
    schedulers::Scheduler,
    state::{HasCorpus, HasMetadata, UsesState},
    Error,
};

#[derive(Default, Clone, Copy, Eq, PartialEq, Debug, Serialize, Deserialize)]
struct TuneableSchedulerMetadata {
    next: Option<CorpusId>,
}

impl_serdeany!(TuneableSchedulerMetadata);

/// Walk the corpus in a queue-like fashion
/// With the specific `set_next` method, we can chose the next corpus entry manually
#[derive(Debug, Clone)]
pub struct TuneableScheduler<S> {
    phantom: PhantomData<S>,
}

impl<S> TuneableScheduler<S>
where
    S: HasMetadata + HasCorpus,
{
    /// Creates a new `TuneableScheduler`
    #[must_use]
    pub fn new(state: &mut S) -> Self {
        if !state.has_metadata::<TuneableSchedulerMetadata>() {
            state.add_metadata(TuneableSchedulerMetadata::default());
        }
        Self {
            phantom: PhantomData,
        }
    }

    fn metadata_mut(state: &mut S) -> &mut TuneableSchedulerMetadata {
        state
            .metadata_mut()
            .get_mut::<TuneableSchedulerMetadata>()
            .unwrap()
    }

    fn metadata(state: &S) -> &TuneableSchedulerMetadata {
        state.metadata().get::<TuneableSchedulerMetadata>().unwrap()
    }

    /// Sets the next corpus id to be used
    pub fn set_next(state: &mut S, next: CorpusId) {
        Self::metadata_mut(state).next = Some(next);
    }

    /// Gets the next set corpus id
    pub fn get_next(state: &S) -> Option<CorpusId> {
        Self::metadata(state).next
    }

    /// Resets this to a queue scheduler
    pub fn reset(state: &mut S) {
        let metadata = Self::metadata_mut(state);
        metadata.next = None;
    }

    /// Gets the current corpus entry id
    pub fn get_current(state: &S) -> CorpusId {
        state
            .corpus()
            .current()
            .unwrap_or_else(|| state.corpus().first().expect("Empty corpus"))
    }
}

impl<S> UsesState for TuneableScheduler<S>
where
    S: UsesInput,
{
    type State = S;
}

impl<S> RemovableScheduler for TuneableScheduler<S> where S: HasCorpus + HasMetadata {}

impl<S> Scheduler for TuneableScheduler<S>
where
    S: HasCorpus + HasMetadata,
{
    fn on_add(&mut self, state: &mut Self::State, idx: CorpusId) -> Result<(), Error> {
        // Set parent id
        let current_idx = *state.corpus().current();
        state
            .corpus()
            .get(idx)?
            .borrow_mut()
            .set_parent_id_optional(current_idx);

        Ok(())
    }

    /// Gets the next entry in the queue
    fn next(&mut self, state: &mut Self::State) -> Result<CorpusId, Error> {
        if state.corpus().count() == 0 {
            return Err(Error::empty("No entries in corpus".to_owned()));
        }
        let id = if let Some(next) = Self::get_next(state) {
            // next was set
            next
        } else if let Some(next) = state.corpus().next(Self::get_current(state)) {
            next
        } else {
            state.corpus().first().unwrap()
        };
        self.set_current_scheduled(state, Some(id))?;
        Ok(id)
    }

    /// Set current fuzzed corpus id and `scheduled_count`
    fn set_current_scheduled(
        &mut self,
        state: &mut Self::State,
        next_idx: Option<CorpusId>,
    ) -> Result<(), Error> {
        *state.corpus_mut().current_mut() = next_idx;
        Ok(())
    }
}
