#!/bin/bash

mkdir -p /etc/mogilefs

cat <<NACHOS > /etc/mogilefs/mogilefsd.conf
db_dsn DBI:mysql:database=mogilefs;host=$MYSQL_PORT_3306_TCP_ADDR;port=$MYSQL_PORT_3306_TCP_PORT
db_user mogile
db_pass blarg
listen 0.0.0.0:7001
NACHOS

cat <<NACHOS > /etc/mogilefs/mogilefs.conf
trackers = 127.0.0.1:7001
db_dsn = DBI:mysql:database=mogilefs;host=$MYSQL_PORT_3306_TCP_ADDR;port=$MYSQL_PORT_3306_TCP_PORT
db_user = mogile
db_pass = blarg
NACHOS

for i in $(seq 1 10); do
    sleep 2
    echo "Running mogdbsetup (attempt $i)..."
    mogdbsetup --verbose --yes \
               --dbhost=$MYSQL_PORT_3306_TCP_ADDR \
               --dbport=$MYSQL_PORT_3306_TCP_PORT \
               --dbrootuser=root \
               --dbuser=mogile \
               --dbpass=blarg \
               --dbname=mogilefs

    if [ $? = 0 ]; then
        echo "Success!"
        break
    fi
done

exec mogilefsd
