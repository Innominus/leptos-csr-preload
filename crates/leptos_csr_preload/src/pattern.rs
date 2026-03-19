use crate::PreloadError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutePattern {
    segments: Vec<RouteSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RouteSegment {
    Static(String),
    Param(String),
    OptionalParam(String),
    Splat(String),
}

impl RoutePattern {
    pub fn parse(input: &str) -> Result<Self, PreloadError> {
        let trimmed = input.trim();
        if trimmed.is_empty() || trimmed == "/" {
            return Ok(Self {
                segments: Vec::new(),
            });
        }

        let mut segments = Vec::new();
        for raw in trimmed.trim_start_matches('/').split('/') {
            if raw.is_empty() {
                continue;
            }

            let segment = if let Some(name) = raw.strip_prefix(':') {
                if let Some(name) = name.strip_suffix('?') {
                    if name.is_empty() {
                        return Err(PreloadError::InvalidRoutePattern {
                            pattern: trimmed.to_string(),
                            reason: "optional parameters must have a name",
                        });
                    }
                    RouteSegment::OptionalParam(name.to_string())
                } else {
                    if name.is_empty() {
                        return Err(PreloadError::InvalidRoutePattern {
                            pattern: trimmed.to_string(),
                            reason: "parameters must have a name",
                        });
                    }
                    RouteSegment::Param(name.to_string())
                }
            } else if let Some(name) = raw.strip_prefix('*') {
                if name.is_empty() {
                    return Err(PreloadError::InvalidRoutePattern {
                        pattern: trimmed.to_string(),
                        reason: "splats must have a name",
                    });
                }
                RouteSegment::Splat(name.to_string())
            } else {
                RouteSegment::Static(raw.to_string())
            };

            segments.push(segment);
        }

        Ok(Self { segments })
    }

    pub fn matches(&self, path: &str) -> bool {
        let normalized = path.split('?').next().unwrap_or(path);
        let parts: Vec<&str> = normalized
            .trim_start_matches('/')
            .split('/')
            .filter(|segment| !segment.is_empty())
            .collect();
        self.matches_from(0, 0, &parts)
    }

    pub fn specificity(&self) -> (usize, usize, usize, usize) {
        let static_count = self
            .segments
            .iter()
            .filter(|segment| matches!(segment, RouteSegment::Static(_)))
            .count();
        let param_count = self
            .segments
            .iter()
            .filter(|segment| matches!(segment, RouteSegment::Param(_)))
            .count();
        let optional_count = self
            .segments
            .iter()
            .filter(|segment| matches!(segment, RouteSegment::OptionalParam(_)))
            .count();
        let splat_count = self
            .segments
            .iter()
            .filter(|segment| matches!(segment, RouteSegment::Splat(_)))
            .count();

        (
            static_count,
            param_count,
            usize::MAX - optional_count,
            usize::MAX - splat_count,
        )
    }

    fn matches_from(&self, pattern_index: usize, path_index: usize, parts: &[&str]) -> bool {
        if pattern_index == self.segments.len() {
            return path_index == parts.len();
        }

        match &self.segments[pattern_index] {
            RouteSegment::Static(expected) => {
                parts
                    .get(path_index)
                    .is_some_and(|segment| *segment == expected)
                    && self.matches_from(pattern_index + 1, path_index + 1, parts)
            }
            RouteSegment::Param(_) => {
                parts.get(path_index).is_some()
                    && self.matches_from(pattern_index + 1, path_index + 1, parts)
            }
            RouteSegment::OptionalParam(_) => {
                self.matches_from(pattern_index + 1, path_index, parts)
                    || (parts.get(path_index).is_some()
                        && self.matches_from(pattern_index + 1, path_index + 1, parts))
            }
            RouteSegment::Splat(_) => (path_index..=parts.len())
                .any(|next| self.matches_from(pattern_index + 1, next, parts)),
        }
    }
}
