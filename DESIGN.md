This implementation scans order by order starting from the top. Each price includes a FIFO with the orders.

Try using a skip_list where the elements are ordered by the price and contain a struct with the FIFO list plus extra metrics as the total amount in that level.

Explore levels extended with additional fields as total quantity.