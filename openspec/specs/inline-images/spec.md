# inline-images Specification

## Purpose

Defines how catmd displays Markdown images inline: choosing the best available terminal image protocol, fitting images to the terminal width, and degrading gracefully when no graphical protocol exists.

## Requirements

### Requirement: Deterministic backend selection
The system SHALL select an image backend in this fixed order: Kitty Graphics Protocol, SIXEL, Unicode half-block, text placeholder. It SHALL pick the first backend the terminal supports, and the same environment SHALL always yield the same choice.

#### Scenario: Kitty available
- **WHEN** the terminal reports Kitty graphics support
- **THEN** the Kitty backend is used even if SIXEL is also supported

#### Scenario: Only SIXEL available
- **WHEN** the terminal supports SIXEL but not Kitty graphics
- **THEN** the SIXEL backend is used

#### Scenario: No graphical protocol
- **WHEN** the terminal supports neither Kitty graphics nor SIXEL
- **THEN** the half-block backend is used when color output is enabled, otherwise the text placeholder

### Requirement: Images only render graphically on a terminal
In automatic mode the system SHALL emit image output only when standard output is a terminal. A terminal without color output SHALL get the placeholder box; a non-terminal SHALL get the plain `[image: <alt>] (<src>)` label. An explicit `--images` mode SHALL override this gate.

#### Scenario: Piped output
- **WHEN** output is piped and the document contains an image
- **THEN** the output contains no graphics escape sequences and contains the plain image label

#### Scenario: Explicit mode overrides the gate
- **WHEN** output is piped and `--images placeholder` is given
- **THEN** the image is rendered as the placeholder box

### Requirement: Images fit the content width
The system SHALL scale an image down proportionally, preserving aspect ratio, so its rendered width never exceeds the available content width. It SHALL NOT scale images up beyond their natural size.

#### Scenario: Wider than the terminal
- **WHEN** an image's natural width exceeds the available content width
- **THEN** it is rendered at the content width with its original aspect ratio

#### Scenario: Narrower than the terminal
- **WHEN** an image's natural width is smaller than the available content width
- **THEN** it is rendered at its natural size

#### Scenario: Nested content
- **WHEN** an image appears inside a block quote or list item
- **THEN** its width is limited to the width remaining after indentation and gutters

### Requirement: Images occupy whole layout rows
The system SHALL treat each rendered image as a block that occupies a whole number of rows and columns in the document flow, with no line wrapping, clipping, or horizontal overflow.

#### Scenario: Surrounding text
- **WHEN** an image sits between two paragraphs
- **THEN** the paragraphs keep their normal spacing and the image's reserved rows contain no other text

### Requirement: Unsupported-terminal placeholder
When no graphical or cell-based image rendering is available, the system SHALL render a visible box containing the alt text when provided, the image source, the original dimensions when known, and a note that image display is unsupported.

#### Scenario: Full metadata
- **WHEN** `![Architecture Diagram](diagrams/rendering.png)` refers to a readable 1920x1080 image and no image backend is available
- **THEN** a box shows `Architecture Diagram`, `diagrams/rendering.png`, `1920×1080`, and `Image display unsupported`

#### Scenario: Missing alt and unknown size
- **WHEN** an image has no alt text and cannot be decoded
- **THEN** the box omits the alt and dimension lines and still shows the source and the unsupported note

#### Scenario: Box width
- **WHEN** the box content is wider than the content width
- **THEN** the box is narrowed and its lines truncated so it never exceeds the content width

### Requirement: Half-block fallback
The system SHALL render an image with Unicode half-block cells and ANSI foreground and background colors when color output is enabled and no graphical protocol is available, fitting the content width and preserving aspect ratio.

#### Scenario: Cell rendering
- **WHEN** the half-block backend renders a 1x2 pixel image into a 1-column, 1-row area
- **THEN** the output cell is an upper-half-block whose foreground and background colors are the two vertical pixels

### Requirement: Load failures degrade
The system SHALL render the placeholder box, not fail, when an image source is missing, unreadable, a remote URL, or not decodable.

#### Scenario: Missing file
- **WHEN** an image path does not exist
- **THEN** catmd renders the placeholder box and exits successfully

### Requirement: Relative sources resolve against the document
The system SHALL resolve a relative local image path against the directory of the Markdown file, or the current directory for standard input.

#### Scenario: File input
- **WHEN** `docs/a.md` references `img/x.png`
- **THEN** the image is loaded from `docs/img/x.png`

### Requirement: Layout is recomputed for new geometry
The system SHALL compute image layout solely from the image's natural size, the available width, and the terminal cell geometry, so that a change in width or cell size yields a fresh, correctly fitted layout without re-reading or re-decoding the image.

#### Scenario: Width change
- **WHEN** the same document is rendered at width 80 and then width 40
- **THEN** the second render fits every image to 40 columns and the file is decoded only once

### Requirement: Image caching
The system SHALL cache decoded image data per source and cache encoded backend output keyed by source, rendered size, cell geometry, and backend. A change in rendered size, cell geometry, or backend SHALL invalidate only the encoded output, not the decoded data.

#### Scenario: Repeated image
- **WHEN** the same source appears twice in a document at the same size
- **THEN** it is decoded and encoded once

#### Scenario: Resize
- **WHEN** the available width changes
- **THEN** encoded output is regenerated and decoded data is reused
