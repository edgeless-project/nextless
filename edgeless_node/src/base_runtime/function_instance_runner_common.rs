// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

pub struct TracingContext {
    pub parent_context: opentelemetry::Context,
}
