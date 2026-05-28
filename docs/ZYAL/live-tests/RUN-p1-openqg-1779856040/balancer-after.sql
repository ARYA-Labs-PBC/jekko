PRAGMA foreign_keys=OFF;
BEGIN TRANSACTION;
CREATE TABLE round_robin_cursor (
               provider TEXT NOT NULL,
               model TEXT NOT NULL,
               cursor INTEGER NOT NULL DEFAULT 0,
               PRIMARY KEY (provider, model)
             );
INSERT INTO round_robin_cursor VALUES('jnoccio','jnoccio-router',198);
COMMIT;
