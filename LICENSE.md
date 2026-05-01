MIT License

Copyright (c) 2026-Present Alex <lixd220@gmail.com> (Unterm)
Copyright (c) 2018-Present Wez Furlong (upstream WezTerm engine on which
Unterm is based — see https://github.com/wezterm/wezterm)

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

---

Unterm bundles `JetBrains Mono`, `Fira Code`, `Noto Color Emoji`, and
`Roboto` fonts under the SIL Open Font License 1.1. The license text
is at `assets/fonts/LICENSE_OFL.txt`.

Unterm bundles `Symbols Nerd Font Mono`, built from those icon sets
available at https://github.com/ryanoasis/nerd-fonts that are clearly
distributed under the OFL 1.1. The Pomicons icon set is excluded.

Each Rust crate in this workspace declares its own license in its
`Cargo.toml` (`license = "..."`). Most are MIT; a few mixed-license
crates (e.g. `bidi` carries `MIT AND Unicode-DFS-2016` for the
Unicode bidi data tables) document their additional terms in the
relevant subdirectory.
