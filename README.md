# Reservation

this system is based on the following tyrchen's  reservation project

```shell

cargo add sqlx --features runtime-tokio-rustls --features postgres --features chrono --features uuid -p reservation
cargo install sqlx-cli
pip install pgcli
docker run --privileged=true --name postgres -d -p 15432:5432 -e POSTGRES_PASSWORD=7cOPpA7dnc postgres
pgcli -h 127.0.0.1 -p 15432 -U postgres reservation

sqlx migrate run

sqlx migrate add reservation_trigger -r

 select * from _sqlx_migrations;





grant dba to user;


 pg_dump -s postgres://postgres:123456@localhost:5432/reservation >reservation/fixtures/dump.sql
 https://github.com/tyrchen/rust-lib-template.git
 https://github.com/tyrchen/rust-lib-template

 $ cargo generate --git https://github.com/tyrchen/rust-lib-template
```

[正则表达式](https://regexr.com/)
[pgclient update](https://blog.csdn.net/worldzhy/article/details/106202523)
