use depot::control::standard;
use fast_rich::{Console, Table};
use ipc::Response;

use crate::error::ResponseError;

pub enum CommandResponse {
  Ping,
  QueryPods(Vec<(u32, String)>),
}

impl Response for CommandResponse {
  type Error = ResponseError;

  fn from_framed(framed: ipc::Framed) -> Result<Self, Self::Error> {
    match framed.kind {
      standard::PING_CODE => Ok(Self::Ping),
      standard::QUERY_POD_CODE => {
        let payload = framed.payload;
        let res = standard::into_pod_list(payload)?;
        Ok(Self::QueryPods(res))
      },
      other => Err(ResponseError::UnknownKindCode(other)),
    }
  }

  fn handle(&mut self) -> Result<(), Self::Error> {
    match self {
      Self::Ping => println!("Pong"),
      Self::QueryPods(pods) => {
        let console = Console::new();

        let mut table = Table::new();
        table.add_column("Id");
        table.add_column("Name");
        table.add_column("Status");
        for (id, name) in pods {
          table.add_row_strs(&[&id.to_string(), name.as_ref(), "Running"]);
        }

        console.print_renderable(&table);
      },
    }

    Ok(())
  }
}
