//! Shared hand-drawn Poly icons for chrome UI (quick actions, popup menu).
//! Vector polys stay crisp at any dpi and have zero font dependency.

use crate::customglyph::{BlockAlpha, BlockCoord, Poly, PolyCommand, PolyStyle};

pub const ICON_TREE: &[Poly] = &[
    Poly {
        path: &[
            PolyCommand::MoveTo(BlockCoord::Frac(1, 4), BlockCoord::Frac(1, 8)),
            PolyCommand::LineTo(BlockCoord::Frac(1, 4), BlockCoord::Frac(7, 8)),
        ],
        intensity: BlockAlpha::Full,
        style: PolyStyle::Outline,
    },
    Poly {
        path: &[
            PolyCommand::MoveTo(BlockCoord::Frac(1, 4), BlockCoord::Frac(1, 4)),
            PolyCommand::LineTo(BlockCoord::Frac(7, 8), BlockCoord::Frac(1, 4)),
        ],
        intensity: BlockAlpha::Full,
        style: PolyStyle::Outline,
    },
    Poly {
        path: &[
            PolyCommand::MoveTo(BlockCoord::Frac(1, 4), BlockCoord::Frac(1, 2)),
            PolyCommand::LineTo(BlockCoord::Frac(7, 8), BlockCoord::Frac(1, 2)),
        ],
        intensity: BlockAlpha::Full,
        style: PolyStyle::Outline,
    },
    Poly {
        path: &[
            PolyCommand::MoveTo(BlockCoord::Frac(1, 4), BlockCoord::Frac(3, 4)),
            PolyCommand::LineTo(BlockCoord::Frac(7, 8), BlockCoord::Frac(3, 4)),
        ],
        intensity: BlockAlpha::Full,
        style: PolyStyle::Outline,
    },
];

pub const ICON_SPLIT: &[Poly] = &[
    Poly {
        path: &[
            PolyCommand::MoveTo(BlockCoord::Frac(1, 8), BlockCoord::Frac(1, 8)),
            PolyCommand::LineTo(BlockCoord::Frac(7, 8), BlockCoord::Frac(1, 8)),
            PolyCommand::LineTo(BlockCoord::Frac(7, 8), BlockCoord::Frac(7, 8)),
            PolyCommand::LineTo(BlockCoord::Frac(1, 8), BlockCoord::Frac(7, 8)),
            PolyCommand::Close,
        ],
        intensity: BlockAlpha::Full,
        style: PolyStyle::Outline,
    },
    Poly {
        path: &[
            PolyCommand::MoveTo(BlockCoord::Frac(1, 2), BlockCoord::Frac(1, 8)),
            PolyCommand::LineTo(BlockCoord::Frac(1, 2), BlockCoord::Frac(7, 8)),
        ],
        intensity: BlockAlpha::Full,
        style: PolyStyle::Outline,
    },
];

pub const ICON_FOLDER: &[Poly] = &[Poly {
    path: &[
        PolyCommand::MoveTo(BlockCoord::Frac(1, 8), BlockCoord::Frac(1, 4)),
        PolyCommand::LineTo(BlockCoord::Frac(3, 8), BlockCoord::Frac(1, 4)),
        PolyCommand::LineTo(BlockCoord::Frac(1, 2), BlockCoord::Frac(3, 8)),
        PolyCommand::LineTo(BlockCoord::Frac(7, 8), BlockCoord::Frac(3, 8)),
        PolyCommand::LineTo(BlockCoord::Frac(7, 8), BlockCoord::Frac(3, 4)),
        PolyCommand::LineTo(BlockCoord::Frac(1, 8), BlockCoord::Frac(3, 4)),
        PolyCommand::Close,
    ],
    intensity: BlockAlpha::Full,
    style: PolyStyle::Outline,
}];

pub const ICON_SLIDERS: &[Poly] = &[
    Poly {
        path: &[
            PolyCommand::MoveTo(BlockCoord::Frac(1, 8), BlockCoord::Frac(1, 4)),
            PolyCommand::LineTo(BlockCoord::Frac(7, 8), BlockCoord::Frac(1, 4)),
        ],
        intensity: BlockAlpha::Full,
        style: PolyStyle::Outline,
    },
    Poly {
        path: &[
            PolyCommand::MoveTo(BlockCoord::Frac(5, 8), BlockCoord::Frac(1, 8)),
            PolyCommand::LineTo(BlockCoord::Frac(5, 8), BlockCoord::Frac(3, 8)),
        ],
        intensity: BlockAlpha::Full,
        style: PolyStyle::Outline,
    },
    Poly {
        path: &[
            PolyCommand::MoveTo(BlockCoord::Frac(1, 8), BlockCoord::Frac(1, 2)),
            PolyCommand::LineTo(BlockCoord::Frac(7, 8), BlockCoord::Frac(1, 2)),
        ],
        intensity: BlockAlpha::Full,
        style: PolyStyle::Outline,
    },
    Poly {
        path: &[
            PolyCommand::MoveTo(BlockCoord::Frac(3, 8), BlockCoord::Frac(3, 8)),
            PolyCommand::LineTo(BlockCoord::Frac(3, 8), BlockCoord::Frac(5, 8)),
        ],
        intensity: BlockAlpha::Full,
        style: PolyStyle::Outline,
    },
    Poly {
        path: &[
            PolyCommand::MoveTo(BlockCoord::Frac(1, 8), BlockCoord::Frac(3, 4)),
            PolyCommand::LineTo(BlockCoord::Frac(7, 8), BlockCoord::Frac(3, 4)),
        ],
        intensity: BlockAlpha::Full,
        style: PolyStyle::Outline,
    },
    Poly {
        path: &[
            PolyCommand::MoveTo(BlockCoord::Frac(11, 16), BlockCoord::Frac(5, 8)),
            PolyCommand::LineTo(BlockCoord::Frac(11, 16), BlockCoord::Frac(7, 8)),
        ],
        intensity: BlockAlpha::Full,
        style: PolyStyle::Outline,
    },
];

pub const ICON_PLUS: &[Poly] = &[
    Poly {
        path: &[
            PolyCommand::MoveTo(BlockCoord::Frac(1, 2), BlockCoord::Frac(1, 8)),
            PolyCommand::LineTo(BlockCoord::Frac(1, 2), BlockCoord::Frac(7, 8)),
        ],
        intensity: BlockAlpha::Full,
        style: PolyStyle::Outline,
    },
    Poly {
        path: &[
            PolyCommand::MoveTo(BlockCoord::Frac(1, 8), BlockCoord::Frac(1, 2)),
            PolyCommand::LineTo(BlockCoord::Frac(7, 8), BlockCoord::Frac(1, 2)),
        ],
        intensity: BlockAlpha::Full,
        style: PolyStyle::Outline,
    },
];

/// Magnifier approximated as an octagonal ring + handle.
pub const ICON_SEARCH: &[Poly] = &[
    Poly {
        path: &[
            PolyCommand::MoveTo(BlockCoord::Frac(3, 16), BlockCoord::Frac(5, 16)),
            PolyCommand::LineTo(BlockCoord::Frac(5, 16), BlockCoord::Frac(3, 16)),
            PolyCommand::LineTo(BlockCoord::Frac(8, 16), BlockCoord::Frac(3, 16)),
            PolyCommand::LineTo(BlockCoord::Frac(10, 16), BlockCoord::Frac(5, 16)),
            PolyCommand::LineTo(BlockCoord::Frac(10, 16), BlockCoord::Frac(8, 16)),
            PolyCommand::LineTo(BlockCoord::Frac(8, 16), BlockCoord::Frac(10, 16)),
            PolyCommand::LineTo(BlockCoord::Frac(5, 16), BlockCoord::Frac(10, 16)),
            PolyCommand::LineTo(BlockCoord::Frac(3, 16), BlockCoord::Frac(8, 16)),
            PolyCommand::Close,
        ],
        intensity: BlockAlpha::Full,
        style: PolyStyle::Outline,
    },
    Poly {
        path: &[
            PolyCommand::MoveTo(BlockCoord::Frac(10, 16), BlockCoord::Frac(10, 16)),
            PolyCommand::LineTo(BlockCoord::Frac(14, 16), BlockCoord::Frac(14, 16)),
        ],
        intensity: BlockAlpha::Full,
        style: PolyStyle::Outline,
    },
];

/// Prompt glyph: chevron + underscore (terminal command palette).
pub const ICON_PROMPT: &[Poly] = &[
    Poly {
        path: &[
            PolyCommand::MoveTo(BlockCoord::Frac(1, 8), BlockCoord::Frac(1, 4)),
            PolyCommand::LineTo(BlockCoord::Frac(7, 16), BlockCoord::Frac(1, 2)),
            PolyCommand::LineTo(BlockCoord::Frac(1, 8), BlockCoord::Frac(3, 4)),
        ],
        intensity: BlockAlpha::Full,
        style: PolyStyle::Outline,
    },
    Poly {
        path: &[
            PolyCommand::MoveTo(BlockCoord::Frac(9, 16), BlockCoord::Frac(7, 8)),
            PolyCommand::LineTo(BlockCoord::Frac(7, 8), BlockCoord::Frac(7, 8)),
        ],
        intensity: BlockAlpha::Full,
        style: PolyStyle::Outline,
    },
];

/// Record: small filled block (the cursor itself).
pub const ICON_RECORD: &[Poly] = &[Poly {
    path: &[
        PolyCommand::MoveTo(BlockCoord::Frac(5, 16), BlockCoord::Frac(5, 16)),
        PolyCommand::LineTo(BlockCoord::Frac(11, 16), BlockCoord::Frac(5, 16)),
        PolyCommand::LineTo(BlockCoord::Frac(11, 16), BlockCoord::Frac(11, 16)),
        PolyCommand::LineTo(BlockCoord::Frac(5, 16), BlockCoord::Frac(11, 16)),
        PolyCommand::Close,
    ],
    intensity: BlockAlpha::Full,
    style: PolyStyle::Fill,
}];

/// Export: up-right arrow.
pub const ICON_EXPORT: &[Poly] = &[
    Poly {
        path: &[
            PolyCommand::MoveTo(BlockCoord::Frac(1, 4), BlockCoord::Frac(3, 4)),
            PolyCommand::LineTo(BlockCoord::Frac(3, 4), BlockCoord::Frac(1, 4)),
        ],
        intensity: BlockAlpha::Full,
        style: PolyStyle::Outline,
    },
    Poly {
        path: &[
            PolyCommand::MoveTo(BlockCoord::Frac(3, 8), BlockCoord::Frac(1, 4)),
            PolyCommand::LineTo(BlockCoord::Frac(3, 4), BlockCoord::Frac(1, 4)),
            PolyCommand::LineTo(BlockCoord::Frac(3, 4), BlockCoord::Frac(5, 8)),
        ],
        intensity: BlockAlpha::Full,
        style: PolyStyle::Outline,
    },
];
