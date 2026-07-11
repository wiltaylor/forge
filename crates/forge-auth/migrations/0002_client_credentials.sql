-- Machine-to-machine (client_credentials) support: roles carried by the client
-- itself, emitted verbatim in tokens where the client is its own subject.
ALTER TABLE clients ADD COLUMN client_roles TEXT NOT NULL DEFAULT '[]';
