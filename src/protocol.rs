pub mod generated {
    include!(concat!(env!("OUT_DIR"), "/mimi_protocol.rs"));
}

pub use generated::*;

use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis() as u64
}

pub mod builders {
    use super::*;
    use flatbuffers::{FlatBufferBuilder, WIPOffset};

    pub struct MessageBuilder<'a> {
        builder: &'a mut FlatBufferBuilder<'a>,
    }

    impl<'a> MessageBuilder<'a> {
        pub fn new(builder: &'a mut FlatBufferBuilder<'a>) -> Self {
            Self { builder }
        }

        pub fn create_user_input(
            &mut self,
            user_message: &str,
            user_id: &str,
            session_id: &str,
            channel: &str,
        ) -> WIPOffset<UserInput> {
            let msg = self.builder.create_string(user_message);
            let uid = self.builder.create_string(user_id);
            let sid = self.builder.create_string(session_id);
            let ch = self.builder.create_string(channel);

            UserInput::create(
                self.builder,
                &UserInputArgs {
                    user_message: Some(msg),
                    user_id: Some(uid),
                    session_id: Some(sid),
                    channel: Some(ch),
                },
            )
        }

        pub fn create_trace_context(
            &mut self,
            correlation_id: &str,
            source_module: &str,
        ) -> WIPOffset<TraceContext> {
            let cid = self.builder.create_string(correlation_id);
            let src = self.builder.create_string(source_module);

            TraceContext::create(
                self.builder,
                &TraceContextArgs {
                    correlation_id: Some(cid),
                    timestamp_ms: now_ms(),
                    source_module: Some(src),
                    parent_id: None,
                    span_id: None,
                },
            )
        }

        pub fn create_mood(
            &mut self,
            curiosity: f32,
            confidence: f32,
            frustration: f32,
            engagement: f32,
        ) -> WIPOffset<Mood> {
            Mood::create(
                self.builder,
                &MoodArgs {
                    curiosity,
                    confidence,
                    frustration,
                    engagement,
                },
            )
        }

        pub fn create_intent_classified(
            &mut self,
            intent_type: IntentType,
            confidence: f32,
            is_social: bool,
            complexity: ComplexityEstimate,
        ) -> WIPOffset<IntentClassified> {
            IntentClassified::create(
                self.builder,
                &IntentClassifiedArgs {
                    intent_type,
                    confidence,
                    entity_tags: None,
                    is_social,
                    estimated_complexity: complexity,
                    raw_intent_text: None,
                },
            )
        }

        pub fn create_task_execution(
            &mut self,
            task_id: &str,
            skill_id: &str,
            skill_name: &str,
            priority: Priority,
            timeout_ms: u32,
        ) -> WIPOffset<TaskExecution> {
            let tid = self.builder.create_string(task_id);
            let sid = self.builder.create_string(skill_id);
            let sname = self.builder.create_string(skill_name);

            TaskExecution::create(
                self.builder,
                &TaskExecutionArgs {
                    task_id: Some(tid),
                    skill_id: Some(sid),
                    skill_name: Some(sname),
                    priority,
                    parameters: None,
                    timeout_ms,
                    routing_decision: None,
                },
            )
        }

        pub fn create_execution_result(
            &mut self,
            task_id: &str,
            success: bool,
            status: ResponseStatus,
            output: &str,
            execution_time_ms: u32,
        ) -> WIPOffset<ExecutionResult> {
            let tid = self.builder.create_string(task_id);
            let out = self.builder.create_string(output);

            ExecutionResult::create(
                self.builder,
                &ExecutionResultArgs {
                    task_id: Some(tid),
                    success,
                    status,
                    output: Some(out),
                    error: None,
                    execution_time_ms,
                    tokens_consumed: 0,
                    error_type: None,
                },
            )
        }
    }
}

pub mod validation {
    use super::*;

    pub fn validate_message(msg: &Message) -> Result<(), String> {
        if msg.version() == 0 {
            return Err("Invalid protocol version".to_string());
        }

        if msg.trace().is_none() {
            return Err("Missing trace context".to_string());
        }

        if msg.body().is_none() {
            return Err("Missing message body".to_string());
        }

        Ok(())
    }

    pub fn validate_intent_classified(intent: &IntentClassified) -> Result<(), String> {
        if !(0.0..=1.0).contains(&intent.confidence()) {
            return Err("Confidence must be [0, 1]".to_string());
        }
        Ok(())
    }

    pub fn validate_mood(mood: &Mood) -> Result<(), String> {
        let values = [
            mood.curiosity(),
            mood.confidence(),
            mood.frustration(),
            mood.engagement(),
        ];

        for &val in &values {
            if !(0.0..=1.0).contains(&val) {
                return Err("Mood values must be [0, 1]".to_string());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flatbuffers::FlatBufferBuilder;

    #[test]
    fn test_message_builder_user_input() {
        let mut builder = FlatBufferBuilder::new(1024);
        let mut msg_builder = builders::MessageBuilder::new(&mut builder);

        let _user_input = msg_builder.create_user_input("Hello Mimi", "user123", "sess456", "cli");

        assert!(true);
    }

    #[test]
    fn test_mood_validation() {
        let mut builder = FlatBufferBuilder::new(256);
        let mood = builders::MessageBuilder::new(&mut builder).create_mood(0.5, 0.7, 0.2, 0.8);

        builder.finish(mood, None);
        let buf = builder.finished_data();

        let root = flatbuffers::root::<Mood>(buf).expect("Failed to deserialize");
        assert!(validation::validate_mood(&root).is_ok());
    }

    #[test]
    fn test_invalid_mood_validation() {
        let mut builder = FlatBufferBuilder::new(256);
        let mood = builders::MessageBuilder::new(&mut builder).create_mood(1.5, 0.7, 0.2, 0.8);

        builder.finish(mood, None);
        let buf = builder.finished_data();

        let root = flatbuffers::root::<Mood>(buf).expect("Failed to deserialize");
        assert!(validation::validate_mood(&root).is_err());
    }
}
