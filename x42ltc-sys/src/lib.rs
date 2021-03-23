// x42ltc-sys: src/lib.rs
//
// Copyright 2019-2020 Johannes Maibaum <jmaibaum@gmail.com>
//
// This file is free software; you can redistribute it and/or modify it
// under the terms of the GNU Lesser General Public License as
// published by the Free Software Foundation; either version 3 of the
// License, or (at your option) any later version.
//
// This file is distributed in the hope that it will be useful, but
// WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
// Lesser General Public License for more details.
//
// You should have received a copy of the GNU Lesser General Public
// License along with this program.  If not, see <http://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: LGPL-3.0-or-later
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::upper_case_acronyms)]

//! # bindgen commandline
//!
//! The following commandline was used to create the initial bindings:
//!
//! ```console
//! bindgen vendor/src/ltc.h -o src/bindings.rs --with-derive-defaults
//! ```

include!("./bindings.rs");
