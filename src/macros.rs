macro_rules! run {
    ($self:expr, $head:ident $(:: $tail:ident)* $(, $field:ident)*) => {{
        let (sender, receiver) = ::tokio::sync::oneshot::channel();
        let command = $head $(:: $tail)* { sender $(, $field)* };

        $self.commands.send(command)?;

        receiver.await?
    }};
}

pub(crate) use run;
