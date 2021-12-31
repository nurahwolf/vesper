use crate::{
    command::CommandResult,
    message::Message,
    twilight_exports::*,
    waiter::{WaiterReceiver, WaiterSender},
};
use parking_lot::Mutex;

/// Framework context given to all command functions, this struct contains all the necessary
/// items to respond the interaction and access shared data.
pub struct SlashContext<'a, D> {
    pub http_client: &'a Client,
    pub data: &'a D,
    waiters: &'a Mutex<Vec<WaiterSender>>,
    pub interaction: ApplicationCommand,
}

impl<'a, D> Clone for SlashContext<'a, D> {
    fn clone(&self) -> Self {
        SlashContext {
            http_client: &self.http_client,
            data: &self.data,
            waiters: &self.waiters,
            interaction: self.interaction.clone(),
        }
    }
}

impl<'a, D> SlashContext<'a, D> {
    /// Creates a new context.
    pub(crate) fn new(
        http_client: &'a Client,
        data: &'a D,
        waiters: &'a Mutex<Vec<WaiterSender>>,
        interaction: ApplicationCommand,
    ) -> Self {
        Self {
            http_client,
            data,
            waiters,
            interaction,
        }
    }

    /// Responds to the interaction with an empty message to allow to respond later.
    ///
    /// When this method is used [update_response](Self::update_response) has to be used to edit the response.
    pub async fn acknowledge(&self) -> CommandResult {
        self
            .http_client
            .interaction_callback(
                self.interaction.id,
                &self.interaction.token,
                &InteractionResponse::DeferredChannelMessageWithSource(CallbackData {
                    allowed_mentions: None,
                    components: None,
                    content: None,
                    embeds: Vec::new(),
                    flags: None,
                    tts: None,
                }),
            )
            .exec()
            .await?;

        Ok(())
    }

    /// Updates the sent interaction, this method is a shortcut to twilight's
    /// [update_interaction_original](Client::update_interaction_original)
    /// but http is automatically provided.
    pub async fn update_response<F>(
        &'a self,
        fun: F,
    ) -> Result<Message<'a, D>, Box<dyn std::error::Error + Send + Sync>>
    where
        F: FnOnce(UpdateOriginalResponse<'a>) -> UpdateOriginalResponse<'a>,
    {
        let mut update = self
            .http_client
            .update_interaction_original(&self.interaction.token)?;
        update = fun(update);
        Ok(update
            .exec()
            .await?
            .model()
            .await
            .map(|msg| Message::new(&self, msg))?)
    }

    /// Waits for a component interaction which satisfies the given predicate.
    pub fn wait_component<F>(&self, fun: F) -> WaiterReceiver
    where
        F: Fn(&MessageComponentInteraction) -> bool + Send + 'static,
    {
        let (sender, receiver) = WaiterSender::new(fun);
        {
            let mut lock = self.waiters.lock();
            lock.push(sender);
        }
        receiver
    }
}