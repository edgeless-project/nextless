// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

use std::io::Write;

pub struct FileLogger {
    outfile: Option<std::fs::File>,
}

impl Default for FileLogger {
    fn default() -> Self {
        Self::new()
    }
}

impl FileLogger {
    pub fn new() -> Self {
        let outfile = if let Ok(file_name) = std::env::var("EVAL_LOG") {
            let mut f = std::fs::File::create(file_name).unwrap();
            writeln!(f, "node_id,function_id,latency_ns").unwrap();
            Some(f)
        } else {
            None
        };

        Self { outfile }
    }
}

impl super::telemetry_events::EventProcessor for FileLogger {
    fn handle(
        &mut self,
        event: &crate::telemetry::telemetry_events::TelemetryEvent,
        event_tags: &std::collections::BTreeMap<String, String>,
    ) -> crate::telemetry::telemetry_events::TelemetryProcessingResult {
        if let Some(outfile) = &mut self.outfile {
            if let crate::telemetry::telemetry_events::TelemetryEvent::FunctionInvocationCompleted { duration, .. } = event {
                if let (Some(node_id), Some(function_id)) = (event_tags.get("NODE_ID"), event_tags.get("FUNCTION_ID")) {
                    let latency_ns = duration.as_nanos();
                    // This happens in a seperate task so should not affect the performance too much.
                    // If this becomes a problem we might need to introduce some buffering,
                    // e.g., https://doc.rust-lang.org/stable/std/io/struct.BufWriter.html.
                    // https://users.rust-lang.org/t/writing-a-line-to-a-file/64602/2
                    writeln!(outfile, "{node_id},{function_id},{latency_ns}").unwrap();
                } else {
                    panic!("Required tags not present!")
                }

                return crate::telemetry::telemetry_events::TelemetryProcessingResult::PROCESSED;
            }
        }
        crate::telemetry::telemetry_events::TelemetryProcessingResult::PROCESSED
    }
}
