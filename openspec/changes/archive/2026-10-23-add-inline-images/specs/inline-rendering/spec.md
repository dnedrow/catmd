## MODIFIED Requirements

### Requirement: Images render as a labeled placeholder
The system SHALL render an image as `[image: <alt>]` followed by a space and the source in parentheses, or `[image]` when the alt text is empty, whenever inline image rendering is not active (output is not a terminal, or color output is disabled). When inline image rendering is active, the `inline-images` capability governs instead.

#### Scenario: Image with alt text
- **WHEN** a paragraph contains `![Logo](logo.png)` and styling is off
- **THEN** the output contains `[image: Logo] (logo.png)`

#### Scenario: Image without alt text
- **WHEN** a paragraph contains `![](logo.png)` and styling is off
- **THEN** the output contains `[image] (logo.png)`
