/*
 * Copyright (c) Yann Collet, Meta Platforms, Inc.
 * All rights reserved.
 *
 * This source code is licensed under both the BSD-style license (found in the
 * LICENSE file in the root directory of this source tree) and the GPLv2 (found
 * in the COPYING file in the root directory of this source tree).
 * You may select, at your option, one of the above-listed licenses.
 */

#ifndef EXAMPLE_SEQ_PROD_H
#define EXAMPLE_SEQ_PROD_H

/* The public-only rewrite drops the static-only sequence producer fuzz surface.
 * Keep the harness hooks as no-ops so existing fuzz targets still build.
 */
#define FUZZ_SEQ_PROD_SETUP()
#define FUZZ_SEQ_PROD_TEARDOWN()

#endif /* EXAMPLE_SEQ_PROD_H */
