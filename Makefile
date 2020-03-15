.PHONY: shell

shell:
	docker exec -it mariadb-proxy /bin/bash

psql:
	docker exec -it postgres-server psql -U postgres -d testdb

mariadb:
	docker exec -it mariadb-server mysql --password=devpassword

tendermint:
	docker exec -it tendermint-node /bin/bash
