//! Semantic versioning utilities for Kumir 3 libraries.
//!
//! [STABLE] Provides parsing, comparison, and requirement matching for SemVer.
//! Supports full versions (`1.2.3`), ranges (`>=1.0, <2.0`, `^1.5`, `~1.5.0`),
//! and pre-releases (`1.0.0-alpha.1`, `2.0.0-beta`).
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │ Version (SemVer triple)                                         │
//! ├─────────────────────────────────────────────────────────────────┤
//! │ - Parse from string (with prerelease/build)                     │
//! │ - Order and compatibility checks                                │
//! │ - Convenience bumps (next major/minor/patch)                    │
//! └─────────────────────────────────────────────────────────────────┘
//! ┌─────────────────────────────────────────────────────────────────┐
//! │ VersionReq (single comparator)                                  │
//! ├─────────────────────────────────────────────────────────────────┤
//! │ - Exact, range, compatible (^), tilde (~), wildcard (*)         │
//! │ - Matches() against Version                                     │
//! └─────────────────────────────────────────────────────────────────┘
//! ┌─────────────────────────────────────────────────────────────────┐
//! │ VersionSpec (AND of VersionReq)                                 │
//! ├─────────────────────────────────────────────────────────────────┤
//! │ - Multiple comma-separated requirements                         │
//! │ - matches() ensures all constraints hold                        │
//! └─────────────────────────────────────────────────────────────────┘
//! ```

use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

// =============================================================================
//         SECTION: VERSION (SEMVER CORE)
// =============================================================================

/// [STABLE] Semantic Version (SemVer)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Version {
    /// Major version (breaking changes)
    pub major: u32,
    /// Minor version (backward-compatible features)
    pub minor: u32,
    /// Patch version (bug fixes)
    pub patch: u32,
    /// Prerelease tag (alpha, beta, rc.1)
    pub prerelease: Option<String>,
    /// Build metadata
    pub build_meta: Option<String>,
}

impl Version {
    /// Creates a new version without prerelease/build metadata
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
            prerelease: None,
            build_meta: None,
        }
    }

    /// Attaches a prerelease tag
    pub fn with_prerelease(mut self, pre: impl Into<String>) -> Self {
        self.prerelease = Some(pre.into());
        self
    }

    /// Attaches build metadata
    pub fn with_build(mut self, build: impl Into<String>) -> Self {
        self.build_meta = Some(build.into());
        self
    }

    /// Returns true when stable (no prerelease and major > 0)
    pub fn is_stable(&self) -> bool {
        self.prerelease.is_none() && self.major > 0
    }

    /// Returns true when this is a prerelease version
    pub fn is_prerelease(&self) -> bool {
        self.prerelease.is_some()
    }

    /// Checks caret compatibility (^) with another version.
    /// ^1.2.3 == >=1.2.3, <2.0.0
    pub fn is_compatible(&self, other: &Version) -> bool {
        if self.major != other.major {
            return false;
        }
        if self.major == 0 {
            // For 0.x.y the minor must also match
            return self.minor == other.minor && self >= other;
        }
        self >= other
    }

    /// Returns the next major version
    pub fn next_major(&self) -> Version {
        Version::new(self.major + 1, 0, 0)
    }

    /// Returns the next minor version
    pub fn next_minor(&self) -> Version {
        Version::new(self.major, self.minor + 1, 0)
    }

    /// Returns the next patch version
    pub fn next_patch(&self) -> Version {
        Version::new(self.major, self.minor, self.patch + 1)
    }

    /// Parses a version string (convenience wrapper around FromStr)
    pub fn parse(s: &str) -> Result<Self, VersionParseError> {
        s.parse()
    }
}

impl Default for Version {
    fn default() -> Self {
        Self::new(1, 0, 0)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(pre) = &self.prerelease {
            write!(f, "-{}", pre)?;
        }
        if let Some(build) = &self.build_meta {
            write!(f, "+{}", build)?;
        }
        Ok(())
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.major.cmp(&other.major) {
            Ordering::Equal => {}
            ord => return ord,
        }
        match self.minor.cmp(&other.minor) {
            Ordering::Equal => {}
            ord => return ord,
        }
        match self.patch.cmp(&other.patch) {
            Ordering::Equal => {}
            ord => return ord,
        }
        // Prerelease versions are lower than stable
        match (&self.prerelease, &other.prerelease) {
            (None, None) => Ordering::Equal,
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (Some(a), Some(b)) => compare_prerelease(a, b),
        }
    }
}

/// Compares prerelease identifiers per SemVer rules
fn compare_prerelease(a: &str, b: &str) -> Ordering {
    let a_parts: Vec<&str> = a.split('.').collect();
    let b_parts: Vec<&str> = b.split('.').collect();

    for (ap, bp) in a_parts.iter().zip(b_parts.iter()) {
        let cmp = match (ap.parse::<u32>(), bp.parse::<u32>()) {
            (Ok(an), Ok(bn)) => an.cmp(&bn),
            (Ok(_), Err(_)) => Ordering::Less,
            (Err(_), Ok(_)) => Ordering::Greater,
            (Err(_), Err(_)) => ap.cmp(bp),
        };
        if cmp != Ordering::Equal {
            return cmp;
        }
    }
    a_parts.len().cmp(&b_parts.len())
}

impl FromStr for Version {
    type Err = VersionParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();

        // Strip leading 'v' if present
        let s = s.strip_prefix('v').unwrap_or(s);

        // Split into main part and build metadata
        let (main, build_meta) = match s.split_once('+') {
            Some((m, b)) => (m, Some(b.to_string())),
            None => (s, None),
        };

        // Split into version and prerelease
        let (version_str, prerelease) = match main.split_once('-') {
            Some((v, p)) => (v, Some(p.to_string())),
            None => (main, None),
        };

        // Parse numeric parts
        let parts: Vec<&str> = version_str.split('.').collect();

        let major = parts
            .first()
            .ok_or_else(|| VersionParseError::new("missing major version"))?
            .parse()
            .map_err(|_| VersionParseError::new("invalid major version format"))?;

        let minor = parts
            .get(1)
            .map(|s| s.parse())
            .transpose()
            .map_err(|_| VersionParseError::new("invalid minor version format"))?
            .unwrap_or(0);

        let patch = parts
            .get(2)
            .map(|s| s.parse())
            .transpose()
            .map_err(|_| VersionParseError::new("invalid patch version format"))?
            .unwrap_or(0);

        Ok(Version {
            major,
            minor,
            patch,
            prerelease,
            build_meta,
        })
    }
}

// =============================================================================
//         SECTION: PARSE ERRORS
// =============================================================================

/// [STABLE] Version parse error
#[derive(Debug, Clone)]
pub struct VersionParseError {
    pub message: String,
}

impl VersionParseError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
        }
    }
}

impl fmt::Display for VersionParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Version parse error: {}", self.message)
    }
}

impl std::error::Error for VersionParseError {}

// =============================================================================
//         SECTION: VERSION REQUIREMENTS
// =============================================================================

/// Comparison operator for version requirements
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionOp {
    /// Exact match: `=1.0.0`
    Exact,
    /// Greater: `>1.0.0`
    Greater,
    /// Greater or equal: `>=1.0.0`
    GreaterEq,
    /// Less: `<1.0.0`
    Less,
    /// Less or equal: `<=1.0.0`
    LessEq,
    /// Compatible versions (caret): `^1.0.0`
    Compatible,
    /// Close versions (tilde): `~1.0.0`
    Tilde,
    /// Any version: `*`
    Wildcard,
}

impl fmt::Display for VersionOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VersionOp::Exact => write!(f, "="),
            VersionOp::Greater => write!(f, ">"),
            VersionOp::GreaterEq => write!(f, ">="),
            VersionOp::Less => write!(f, "<"),
            VersionOp::LessEq => write!(f, "<="),
            VersionOp::Compatible => write!(f, "^"),
            VersionOp::Tilde => write!(f, "~"),
            VersionOp::Wildcard => write!(f, "*"),
        }
    }
}

/// Single version requirement
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionReq {
    pub op: VersionOp,
    pub version: Version,
}

impl VersionReq {
    /// Creates an exact-version requirement
    pub fn exact(version: Version) -> Self {
        Self {
            op: VersionOp::Exact,
            version,
        }
    }

    /// Creates a caret-compatible requirement (^)
    pub fn compatible(version: Version) -> Self {
        Self {
            op: VersionOp::Compatible,
            version,
        }
    }

    /// Checks whether a version satisfies this requirement
    pub fn matches(&self, version: &Version) -> bool {
        match self.op {
            VersionOp::Exact => version == &self.version,
            VersionOp::Greater => version > &self.version,
            VersionOp::GreaterEq => version >= &self.version,
            VersionOp::Less => version < &self.version,
            VersionOp::LessEq => version <= &self.version,
            VersionOp::Compatible => {
                // ^1.2.3 = >=1.2.3, <2.0.0
                // ^0.2.3 = >=0.2.3, <0.3.0
                // ^0.0.3 = >=0.0.3, <0.0.4
                if version < &self.version {
                    return false;
                }
                if self.version.major == 0 {
                    if self.version.minor == 0 {
                        version.major == 0
                            && version.minor == 0
                            && version.patch == self.version.patch
                    } else {
                        version.major == 0 && version.minor == self.version.minor
                    }
                } else {
                    version.major == self.version.major
                }
            }
            VersionOp::Tilde => {
                // ~1.2.3 = >=1.2.3, <1.3.0
                version >= &self.version
                    && version.major == self.version.major
                    && version.minor == self.version.minor
            }
            VersionOp::Wildcard => true,
        }
    }
}

impl fmt::Display for VersionReq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.op {
            VersionOp::Wildcard => write!(f, "*"),
            _ => write!(f, "{}{}", self.op, self.version),
        }
    }
}

impl FromStr for VersionReq {
    type Err = VersionParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();

        if s == "*" {
            return Ok(VersionReq {
                op: VersionOp::Wildcard,
                version: Version::default(),
            });
        }

        // Detect operator and the remaining version string
        let (op, version_str) = if s.starts_with(">=") {
            (VersionOp::GreaterEq, &s[2..])
        } else if s.starts_with("<=") {
            (VersionOp::LessEq, &s[2..])
        } else if s.starts_with('>') {
            (VersionOp::Greater, &s[1..])
        } else if s.starts_with('<') {
            (VersionOp::Less, &s[1..])
        } else if s.starts_with('^') {
            (VersionOp::Compatible, &s[1..])
        } else if s.starts_with('~') {
            (VersionOp::Tilde, &s[1..])
        } else if s.starts_with('=') {
            (VersionOp::Exact, &s[1..])
        } else {
            // No operator means exact match
            (VersionOp::Exact, s)
        };

        let version = version_str.parse()?;
        Ok(VersionReq { op, version })
    }
}

// =============================================================================
//         SECTION: VERSION SPEC (AND OF REQUIREMENTS)
// =============================================================================

/// Composite version spec (comma-separated requirements)
/// Example: `>=1.0, <2.0` or `^1.5`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionSpec {
    /// List of requirements (AND logic)
    pub requirements: Vec<VersionReq>,
}

impl VersionSpec {
    /// Creates a spec that allows any version
    pub fn any() -> Self {
        Self {
            requirements: vec![],
        }
    }

    /// Creates a spec for an exact version
    pub fn exact(version: Version) -> Self {
        Self {
            requirements: vec![VersionReq::exact(version)],
        }
    }

    /// Creates a spec for caret-compatible versions
    pub fn compatible(version: Version) -> Self {
        Self {
            requirements: vec![VersionReq::compatible(version)],
        }
    }

    /// Returns true if the version satisfies all requirements
    pub fn matches(&self, version: &Version) -> bool {
        if self.requirements.is_empty() {
            return true;
        }
        self.requirements.iter().all(|req| req.matches(version))
    }

    /// Adds another requirement (AND)
    pub fn add_requirement(&mut self, req: VersionReq) {
        self.requirements.push(req);
    }

    /// Parses a version spec string (convenience wrapper around FromStr)
    pub fn parse(s: &str) -> Result<Self, String> {
        s.parse().map_err(|e: VersionParseError| e.message)
    }
}

impl Default for VersionSpec {
    fn default() -> Self {
        Self::any()
    }
}

impl fmt::Display for VersionSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.requirements.is_empty() {
            return write!(f, "*");
        }
        let reqs: Vec<String> = self.requirements.iter().map(|r| r.to_string()).collect();
        write!(f, "{}", reqs.join(", "))
    }
}

impl FromStr for VersionSpec {
    type Err = VersionParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();

        if s.is_empty() || s == "*" {
            return Ok(VersionSpec::any());
        }

        let requirements: Result<Vec<VersionReq>, _> =
            s.split(',').map(|part| part.trim().parse()).collect();

        Ok(VersionSpec {
            requirements: requirements?,
        })
    }
}

// =============================================================================
//         SECTION: UNIT TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing() {
        let v: Version = "1.2.3".parse().unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);

        let v: Version = "v2.0.0-alpha.1".parse().unwrap();
        assert_eq!(v.major, 2);
        assert_eq!(v.prerelease, Some("alpha.1".to_string()));

        let v: Version = "1.0".parse().unwrap();
        assert_eq!(v.patch, 0);
    }

    #[test]
    fn test_version_ordering() {
        let v1: Version = "1.0.0".parse().unwrap();
        let v2: Version = "1.0.1".parse().unwrap();
        let v3: Version = "1.1.0".parse().unwrap();
        let v4: Version = "2.0.0".parse().unwrap();

        assert!(v1 < v2);
        assert!(v2 < v3);
        assert!(v3 < v4);

        let alpha: Version = "1.0.0-alpha".parse().unwrap();
        let beta: Version = "1.0.0-beta".parse().unwrap();
        let stable: Version = "1.0.0".parse().unwrap();

        assert!(alpha < beta);
        assert!(beta < stable);
    }

    #[test]
    fn test_version_req() {
        let req: VersionReq = "^1.2.3".parse().unwrap();

        assert!(req.matches(&"1.2.3".parse().unwrap()));
        assert!(req.matches(&"1.9.9".parse().unwrap()));
        assert!(!req.matches(&"2.0.0".parse().unwrap()));
        assert!(!req.matches(&"1.2.2".parse().unwrap()));

        let _req: VersionReq = ">=1.0, <2.0"
            .parse()
            .unwrap_or_else(|_| VersionReq::compatible("1.0.0".parse().unwrap()));
        // Проверка для VersionSpec ниже
    }

    #[test]
    fn test_version_spec() {
        let spec: VersionSpec = ">=1.0.0, <2.0.0".parse().unwrap();

        assert!(spec.matches(&"1.0.0".parse().unwrap()));
        assert!(spec.matches(&"1.5.3".parse().unwrap()));
        assert!(!spec.matches(&"0.9.0".parse().unwrap()));
        assert!(!spec.matches(&"2.0.0".parse().unwrap()));
    }
}
