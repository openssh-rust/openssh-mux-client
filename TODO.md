 - Implement sftp protocol, crewate a sftp `Session` using the following code:
   

   ```
   /// Returns a preconfigured Session that can be used to create sftp processes
   /// on remote machines.
   fn get_sftp_session() -> Session<'static> {
       Session::builder()
           .subsystem(true)
           .term(Cow::Borrowed(""))
           .cmd(Cow::Borrowed("sftp"))
           .build()
   }

   ```
