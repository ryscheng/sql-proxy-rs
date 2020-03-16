.PHONY: shell psql mysql

shell:
	docker exec -it proxy /bin/bash

psql:
	docker exec -it postgres-server psql "postgresql://root:testpassword@proxy:5432/testdb?sslmode=disable"

mysql:
	docker exec -it mariadb-server mysql --host=proxy --user=root --password=testpassword testdb

tendermint:
	docker exec -it tendermint-node /bin/bash
