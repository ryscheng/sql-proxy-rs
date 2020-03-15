.PHONY: shell

shell:
	docker exec -it proxy /bin/bash

psql:
	docker exec -it postgres-server psql --host proxy --username root #--dbname testdb

mysql:
	docker exec -it mariadb-server mysql --host=proxy --user=root --password=devpassword #testdb

tendermint:
	docker exec -it tendermint-node /bin/bash
