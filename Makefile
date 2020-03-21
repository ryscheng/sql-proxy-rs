.PHONY: shell psql mysql

shell:
	docker exec -it proxy /bin/bash

psql:
	docker exec -it postgres-server psql "postgresql://root:testpassword@proxy:5432/testdb?sslmode=disable"
	#docker exec -it postgres-server psql --host proxy --username root "sslmode=disable"

mysql:
	docker exec -it mariadb-server mysql --host=proxy --user=root --password=testpassword testdb

