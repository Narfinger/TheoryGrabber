-- Your SQL goes here
CREATE TABLE papers (
       id Integer PRIMARY KEY NOT NULL,
       title VARCHAR NOT NULL,
       authors VARCHAR NOT NULL,
       published_at BigInt NOT NULL,
       description VARCHAR NOT NULL,
       link VARCHAR NOT NULL
);
