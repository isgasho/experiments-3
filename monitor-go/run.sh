#!/bin/bash -e

<<COMMENT
curl -X GET -L http://localhost:8080

# output
{"prices":[{"denom":"ucad","price":38.97557721941764,"volume":173262376},{"denom":"udkk","price":194.4282990228238,"volume":3812652597},{"denom":"uluna","price":1,"volume":995839805156561},{"denom":"umnt","price":87958.29700098753,"volume":2863527343152836},{"denom":"uusd","price":30.85280498657667,"volume":2401422877141868},{"denom":"uaud","price":42.193895013177425,"volume":2054520680},{"denom":"ugbp","price":22.462384670477142,"volume":754301511},{"denom":"uhkd","price":239.96848926484535,"volume":464784725},{"denom":"usek","price":266.23317362186816,"volume":2697013724},{"denom":"ucny","price":199.40479476154866,"volume":3312155066},{"denom":"ueur","price":26.14482120965,"volume":9380844150},{"denom":"uinr","price":2253.0251894134167,"volume":24817910251},{"denom":"usgd","price":41.506340254051565,"volume":114665786},{"denom":"uthb","price":998.288630284143,"volume":14528538584},{"denom":"uchf","price":28.30430158907546,"volume":2432203612},{"denom":"ujpy","price":3400.7521574024922,"volume":88721524065},{"denom":"ukrw","price":35722.600310120084,"volume":44206393689698703},{"denom":"unok","price":0,"volume":42813504},{"denom":"usdr","price":21.74428563441457,"volume":716461736241423}]}
COMMENT

go run -v .
