//! Shared contracts for detecting, validating, and provisioning the app's core components.

use std::path::PathBuf;

/// A core component that can be found on this machine and proven usable.
pub trait Component: Sized {
    /// The name shown to users in reports.
    const NAME: &'static str;

    /// What detection starts from, such as an override path or a parent component.
    type Hint<'a>;

    /// Facts learned while locating that validation needs.
    type Metadata;

    /// Finds where this component appears to live, without judging whether it is usable.
    fn locate(hint: Self::Hint<'_>) -> Result<Location<Self::Metadata>, Missing>;

    /// Builds the component from a location, or reports the requirements it fails.
    fn validate(location: &Location<Self::Metadata>) -> Result<Self, Vec<Requirement>>;

    /// Locates and validates the component.
    fn detect(hint: Self::Hint<'_>) -> Detection<Self> {
        let location = match Self::locate(hint) {
            Ok(location) => location,
            Err(missing) => return Detection::Missing(missing),
        };

        match Self::validate(&location) {
            Ok(component) => Detection::Ready(component),
            Err(unmet) => Detection::Invalid { location, unmet },
        }
    }
}

/// Where a component was found, how, and what was learned on the way.
#[derive(Debug, Clone)]
pub struct Location<M> {
    pub path: PathBuf,
    pub source: Source,
    pub metadata: M,
}

/// How a location was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// A path the user configured.
    Override,
    /// A path found by searching the usual places.
    Search,
    /// A path the app just created by provisioning.
    Created,
}

/// The outcome of detecting a component.
pub enum Detection<T: Component> {
    Ready(T),
    Invalid {
        location: Location<T::Metadata>,
        unmet: Vec<Requirement>,
    },
    Missing(Missing),
}

impl<T: Component> Detection<T> {
    /// The component, only when it is ready to use.
    pub fn ready(self) -> Option<T> {
        match self {
            Detection::Ready(component) => Some(component),
            _ => None,
        }
    }

    /// A borrow of the component, only when it is ready to use.
    pub fn as_ready(&self) -> Option<&T> {
        match self {
            Detection::Ready(component) => Some(component),
            _ => None,
        }
    }
}

/// Where detection looked before giving up.
#[derive(Debug)]
pub struct Missing {
    pub searched: Vec<PathBuf>,
}

/// A requirement a component failed to meet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requirement {
    pub name: &'static str,
}

/// A component the app can create when it is absent.
pub trait Provision: Sized {
    /// What creation needs, such as a target directory.
    type Options<'a>;

    /// The ways creation can fail.
    type Error: std::error::Error;

    /// Creates the component, or returns the existing one when it is already present.
    fn provision(options: Self::Options<'_>) -> Result<Provisioned<Self>, Self::Error>;
}

/// What a provisioning request did.
pub enum Provisioned<T> {
    Created(T),
    AlreadyPresent(T),
}
