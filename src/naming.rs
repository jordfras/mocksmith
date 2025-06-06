/// Default function to generate mock names.
///
/// This function generates a mock name by stripping common prefixes or suffixes like
/// "Interface", "Ifc", or "I" from the class name and prepending "Mock" to it.
pub fn default_name_mock(class_name: &str) -> String {
    if class_name.ends_with("Interface") {
        format!("Mock{}", class_name.strip_suffix("Interface").unwrap())
    } else if class_name.ends_with("Ifc") {
        format!("Mock{}", class_name.strip_suffix("Ifc").unwrap())
    } else if class_name.starts_with("Interface") {
        format!("Mock{}", class_name.strip_prefix("Interface").unwrap())
    } else if class_name.starts_with("Ifc") {
        format!("Mock{}", class_name.strip_prefix("Ifc").unwrap())
    } else if class_name.starts_with("I")
        && class_name.len() > 1
        && class_name.chars().nth(1).unwrap().is_uppercase()
    {
        format!("Mock{}", class_name.strip_prefix("I").unwrap())
    } else {
        format!("Mock{}", class_name)
    }
}

/// Helper struct to name mocks based on sed style regex replacement.
pub struct SedReplacement {
    regex: regex::Regex,
    name_pattern: String,
}

impl SedReplacement {
    fn new(regex: &str, name_pattern: &str) -> crate::Result<Self> {
        Ok(Self {
            regex: regex::Regex::new(&format!("^{regex}$")).map_err(|err| {
                crate::MocksmithError::InvalidSedReplacement(format!(
                    "Invalid regex for name replacement: {}",
                    err
                ))
            })?,
            name_pattern: name_pattern.to_string(),
        })
    }

    /// Creates a `SedReplacementNamer` from a sed style replacement string, e.g.,
    /// `s/Ifc(.*)/Mock\1/`, to replace the prefix "Ifc" in class names with "Mock".
    /// The regex pattern must match the entire class name.
    pub fn from_sed_replacement(sed_replacement: &str) -> crate::Result<Self> {
        let parts: Vec<&str> = sed_replacement.split('/').collect();
        if !sed_replacement.ends_with('/') || parts.len() != 4 || parts[0] != "s" {
            return Err(crate::MocksmithError::InvalidSedReplacement(format!(
                "Got {}, but expected s/<regex>/<replacement>/",
                sed_replacement
            )));
        }
        Self::new(parts[1], parts[2])
    }

    /// Generates a mock name based on the provided class name using the regex and name
    /// pattern. If the regex does not match, it defaults to prefixing "Mock" to the
    /// class name.
    pub fn mock_name(&self, class_name: &str) -> String {
        let Some(captures) = self.regex.captures(class_name) else {
            return format!("Mock{}", class_name);
        };

        let mut mock_name = self.name_pattern.clone();
        for i in 1..captures.len() {
            mock_name = mock_name.replace(&format!("\\{i}"), captures.get(i).unwrap().as_str());
        }
        mock_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_name_mock() {
        assert_eq!(default_name_mock("MyTypeInterface"), "MockMyType");
        assert_eq!(default_name_mock("MyTypeIfc"), "MockMyType");
        assert_eq!(default_name_mock("InterfaceMyType"), "MockMyType");
        assert_eq!(default_name_mock("IfcMyType"), "MockMyType");
        assert_eq!(default_name_mock("IMyType"), "MockMyType");

        assert_eq!(default_name_mock("MyType"), "MockMyType");
        assert_eq!(default_name_mock("InterestingType"), "MockInterestingType");
        assert_eq!(default_name_mock("I"), "MockI");
    }

    #[test]
    fn sed_namer_replaces_matches() {
        let namer = SedReplacement::from_sed_replacement(r"s/Ifc(.*)/Mock\1/").unwrap();
        assert_eq!(namer.mock_name("IfcMyType"), "MockMyType");
    }

    #[test]
    fn sed_namer_defaults_to_prefix() {
        let namer = SedReplacement::from_sed_replacement(r"s/Ifc(.*)/Mock\1/").unwrap();
        assert_eq!(namer.mock_name("IMyType"), "MockIMyType");
        assert_eq!(namer.mock_name("MyIfcType"), "MockMyIfcType");
    }

    #[test]
    fn invalid_sed_style_causes_error() {
        let result = SedReplacement::from_sed_replacement(r"s/Ifc(.*)/Mock\1");
        assert!(matches!(
            result,
            Err(crate::MocksmithError::InvalidSedReplacement(_))
        ));
        assert_eq!(
            result.err().unwrap().to_string(),
            "Invalid sed style replacement string: \
             Got s/Ifc(.*)/Mock\\1, but expected s/<regex>/<replacement>/"
        );

        let result = SedReplacement::from_sed_replacement(r"s/Ifc(.*/Mock\1/");
        assert!(matches!(
            result,
            Err(crate::MocksmithError::InvalidSedReplacement(_))
        ));
        assert_eq!(
            result.err().unwrap().to_string(),
            format!(
                "{}{}{}{}",
                "Invalid sed style replacement string: \
                 Invalid regex for name replacement: \
                 regex parse error:\n",
                "    ^Ifc(.*$\n",
                "        ^\n",
                "error: unclosed group"
            )
        );
    }
}
