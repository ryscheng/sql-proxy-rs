.PHONY: shell psql mysql

shell:
	docker exec -it proxy /bin/bash

psql:
	docker exec -it postgres-server psql "postgresql://root:testpassword@proxy:5432/testdb?sslmode=disable"

mysql:
	docker exec -it mariadb-server mysql --host=proxy --user=root --password=testpassword testdb

tendermint:
	docker exec -it tendermint-node /bin/bash

mediawiki:
	docker exec -it mediawiki /bin/bash

# for importing data
migrate:
	docker exec -it mariadb-server /bin/bash -c "mysql --user=root --password=devpassword --database=mediawiki < /code/tables.sql"
	docker exec -it mediawiki /bin/bash -c "php /var/www/html/maintenance/importDump.php data.xml"
