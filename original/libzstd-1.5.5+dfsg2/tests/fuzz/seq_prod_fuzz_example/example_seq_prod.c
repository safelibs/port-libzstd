/*
 * Copyright (c) Yann Collet, Meta Platforms, Inc.
 * All rights reserved.
 *
 * This source code is licensed under both the BSD-style license (found in the
 * LICENSE file in the root directory of this source tree) and the GPLv2 (found
 * in the COPYING file in the root directory of this source tree).
 * You may select, at your option, one of the above-listed licenses.
 */

/*
 * The public-only fuzz rewrite no longer exercises the static-only sequence
 * producer surface. Keep a tiny translation unit so the example directory still
 * builds when referenced from external scripts.
 */

int fuzz_seq_prod_example_public_only_placeholder = 0;
