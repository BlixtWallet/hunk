impl AiWorkerRuntime {
    fn maybe_recover_stalled_turns(
        &mut self,
        config: &AiWorkerStartConfig,
        event_tx: &Sender<AiWorkerEvent>,
    ) -> Result<(), CodexIntegrationError> {
        let now = Instant::now();
        self.sync_stream_watches_from_state(now);

        let stalled = self
            .turn_stream_watches
            .values()
            .filter(|watch| !self.turn_is_waiting_for_user_action(&watch.thread_id, &watch.turn_id))
            .filter(|watch| {
                now.duration_since(watch.last_meaningful_activity_at) >= STREAM_STALL_THRESHOLD
            })
            .filter(|watch| {
                watch.last_recovery_at.is_none_or(|last_recovery| {
                    now.duration_since(last_recovery) >= STREAM_STALL_RECOVERY_COOLDOWN
                })
            })
            .map(|watch| {
                (
                    watch.thread_id.clone(),
                    watch.turn_id.clone(),
                    watch.soft_recovery_attempts,
                    now.duration_since(watch.last_meaningful_activity_at),
                )
            })
            .collect::<Vec<_>>();

        let Some((thread_id, turn_id, soft_recovery_attempts, stalled_for)) =
            stalled.into_iter().next()
        else {
            return Ok(());
        };

        tracing::warn!(
            thread_id = thread_id.as_str(),
            turn_id = turn_id.as_str(),
            stalled_for_ms = stalled_for.as_millis() as u64,
            soft_recovery_attempts,
            "detected stalled AI stream"
        );

        if soft_recovery_attempts < STREAM_STALL_MAX_SOFT_RECOVERIES {
            if let Some(watch) = self.turn_stream_watches.get_mut(turn_id.as_str()) {
                watch.last_recovery_at = Some(now);
                watch.last_meaningful_activity_at = now;
                watch.soft_recovery_attempts = watch.soft_recovery_attempts.saturating_add(1);
            }

            tracing::info!(
                thread_id = thread_id.as_str(),
                turn_id = turn_id.as_str(),
                stalled_for_ms = stalled_for.as_millis() as u64,
                next_soft_recovery_attempt = soft_recovery_attempts.saturating_add(1),
                "attempting stalled AI stream recovery via thread snapshot refresh"
            );
            self.send_event(
                event_tx,
                AiWorkerEventPayload::Status(format!(
                    "AI stream stalled for turn {turn_id}. Attempting recovery..."
                )),
            );
            self.load_thread_snapshot(thread_id)?;
            self.emit_snapshot_after_sync(event_tx)?;
            return Ok(());
        }

        if self.transport_kind == AppServerTransportKind::Embedded {
            if let Some(watch) = self.turn_stream_watches.get_mut(turn_id.as_str()) {
                watch.last_recovery_at = Some(now);
                watch.last_meaningful_activity_at = now;
            }

            tracing::warn!(
                thread_id = thread_id.as_str(),
                turn_id = turn_id.as_str(),
                stalled_for_ms = stalled_for.as_millis() as u64,
                soft_recovery_attempts,
                "stall recovery exhausted soft retries; refreshing snapshot without embedded runtime reboot"
            );
            self.send_event(
                event_tx,
                AiWorkerEventPayload::Status(format!(
                    "AI stream is still stalled for turn {turn_id}. Refreshing thread state without restarting the embedded runtime..."
                )),
            );
            self.load_thread_snapshot(thread_id)?;
            self.emit_snapshot_after_sync(event_tx)?;
            return Ok(());
        }

        self.send_event(
            event_tx,
            AiWorkerEventPayload::Status(format!(
                "AI stream is still stalled for turn {turn_id}. Reconnecting transport..."
            )),
        );
        tracing::warn!(
            thread_id = thread_id.as_str(),
            turn_id = turn_id.as_str(),
            stalled_for_ms = stalled_for.as_millis() as u64,
            soft_recovery_attempts,
            "stall recovery exhausted soft retries; reconnecting AI transport"
        );
        self.reconnect_after_transport_failure(config, "recovering stalled AI stream", event_tx)?;
        self.sync_stream_watches_from_state(Instant::now());
        Ok(())
    }

    fn sync_stream_watches_from_state(&mut self, now: Instant) {
        let mut active_watches = BTreeMap::new();
        for turn in self
            .service
            .state()
            .turns
            .values()
            .filter(|turn| turn.status == StateTurnStatus::InProgress)
        {
            let watch = self
                .turn_stream_watches
                .remove(turn.id.as_str())
                .unwrap_or_else(|| TurnStreamWatch {
                    thread_id: turn.thread_id.clone(),
                    turn_id: turn.id.clone(),
                    last_meaningful_activity_at: now,
                    last_recovery_at: None,
                    soft_recovery_attempts: 0,
                });
            active_watches.insert(
                turn.id.clone(),
                TurnStreamWatch {
                    thread_id: turn.thread_id.clone(),
                    turn_id: turn.id.clone(),
                    ..watch
                },
            );
        }
        self.turn_stream_watches = active_watches;
    }

    fn mark_stream_activity_from_notification(
        &mut self,
        notification: &ServerNotification,
        now: Instant,
    ) {
        let Some((thread_id, turn_id)) = notification_turn_identity(notification) else {
            return;
        };
        self.mark_stream_activity(thread_id, turn_id, now);
    }

    fn mark_stream_activity(&mut self, thread_id: &str, turn_id: &str, now: Instant) {
        let watch = self
            .turn_stream_watches
            .entry(turn_id.to_string())
            .or_insert_with(|| TurnStreamWatch {
                thread_id: thread_id.to_string(),
                turn_id: turn_id.to_string(),
                last_meaningful_activity_at: now,
                last_recovery_at: None,
                soft_recovery_attempts: 0,
            });
        watch.thread_id = thread_id.to_string();
        watch.turn_id = turn_id.to_string();
        watch.last_meaningful_activity_at = now;
        watch.last_recovery_at = None;
        watch.soft_recovery_attempts = 0;
    }

    fn turn_is_waiting_for_user_action(&self, thread_id: &str, turn_id: &str) -> bool {
        self.pending_approvals.values().any(|pending| {
            pending.approval.thread_id == thread_id && pending.approval.turn_id == turn_id
        }) || self.pending_user_inputs.values().any(|pending| {
            pending.request.thread_id == thread_id && pending.request.turn_id == turn_id
        })
    }
}
