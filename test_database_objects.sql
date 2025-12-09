-- ====================================================================
-- MySQL Database Objects Test Script
-- This script creates views, functions, procedures, and triggers
-- to test the MySQL to PostgreSQL migration
-- ====================================================================

-- Use your test database
-- USE testing;

-- ====================================================================
-- 1. CREATE TEST TABLES (if they don't exist)
-- ====================================================================

CREATE TABLE IF NOT EXISTS customers (
    customer_id INT AUTO_INCREMENT PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    email VARCHAR(100),
    status ENUM('active', 'inactive', 'pending') DEFAULT 'active',
    total_spent DECIMAL(10,2) DEFAULT 0.00,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS orders (
    order_id INT AUTO_INCREMENT PRIMARY KEY,
    customer_id INT NOT NULL,
    order_date DATETIME DEFAULT CURRENT_TIMESTAMP,
    total_amount DECIMAL(10,2) NOT NULL,
    status VARCHAR(20) DEFAULT 'pending',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    FOREIGN KEY (customer_id) REFERENCES customers(customer_id)
);

CREATE TABLE IF NOT EXISTS products (
    product_id INT AUTO_INCREMENT PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    price DECIMAL(10,2) NOT NULL,
    stock_quantity INT DEFAULT 0,
    discount_percentage INT DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS order_items (
    item_id INT AUTO_INCREMENT PRIMARY KEY,
    order_id INT NOT NULL,
    product_id INT NOT NULL,
    quantity INT NOT NULL,
    unit_price DECIMAL(10,2) NOT NULL,
    FOREIGN KEY (order_id) REFERENCES orders(order_id),
    FOREIGN KEY (product_id) REFERENCES products(product_id)
);

CREATE TABLE IF NOT EXISTS audit_log (
    log_id INT AUTO_INCREMENT PRIMARY KEY,
    table_name VARCHAR(50),
    action VARCHAR(20),
    old_value TEXT,
    new_value TEXT,
    changed_by VARCHAR(50),
    changed_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Insert some test data
INSERT INTO customers (name, email, status, total_spent) VALUES
('John Doe', 'john@example.com', 'active', 1500.00),
('Jane Smith', 'jane@example.com', 'active', 2300.50),
('Bob Wilson', 'bob@example.com', 'inactive', 0.00),
('Alice Brown', 'alice@example.com', 'pending', 450.00)
ON DUPLICATE KEY UPDATE name = name;

INSERT INTO products (name, description, price, stock_quantity, discount_percentage) VALUES
('Laptop', 'High-performance laptop', 1200.00, 10, 10),
('Mouse', 'Wireless mouse', 25.00, 50, 0),
('Keyboard', 'Mechanical keyboard', 80.00, 30, 5),
('Monitor', '27-inch 4K monitor', 400.00, 15, 15)
ON DUPLICATE KEY UPDATE name = name;

-- ====================================================================
-- 2. CREATE VIEWS
-- ====================================================================

-- View 1: Customer Summary
DROP VIEW IF EXISTS customer_summary;
CREATE VIEW customer_summary AS
SELECT 
    c.customer_id,
    c.name,
    c.email,
    c.status,
    COUNT(o.order_id) as order_count,
    IFNULL(SUM(o.total_amount), 0) as total_spent,
    c.created_at
FROM customers c
LEFT JOIN orders o ON c.customer_id = o.customer_id
GROUP BY c.customer_id, c.name, c.email, c.status, c.created_at;

-- View 2: Active Customers
DROP VIEW IF EXISTS active_customers;
CREATE VIEW active_customers AS
SELECT 
    customer_id,
    name,
    email,
    total_spent
FROM customers
WHERE status = 'active';

-- View 3: Product Inventory
DROP VIEW IF EXISTS product_inventory;
CREATE VIEW product_inventory AS
SELECT 
    product_id,
    name,
    price,
    stock_quantity,
    discount_percentage,
    ROUND(price * (1 - discount_percentage / 100), 2) as discounted_price
FROM products
WHERE stock_quantity > 0;

-- View 4: Order Details (complex view with multiple joins)
DROP VIEW IF EXISTS order_details;
CREATE VIEW order_details AS
SELECT 
    o.order_id,
    o.order_date,
    c.name as customer_name,
    c.email as customer_email,
    p.name as product_name,
    oi.quantity,
    oi.unit_price,
    (oi.quantity * oi.unit_price) as line_total,
    o.status as order_status
FROM orders o
INNER JOIN customers c ON o.customer_id = c.customer_id
INNER JOIN order_items oi ON o.order_id = oi.order_id
INNER JOIN products p ON oi.product_id = p.product_id;

-- View 5: Low Stock Products
DROP VIEW IF EXISTS low_stock_products;
CREATE VIEW low_stock_products AS
SELECT 
    product_id,
    name,
    stock_quantity,
    price,
    CONCAT(name, ' (', stock_quantity, ' left)') as alert_message
FROM products
WHERE stock_quantity < 20;

-- ====================================================================
-- 3. CREATE FUNCTIONS
-- ====================================================================

-- Function 1: Calculate Discount
DROP FUNCTION IF EXISTS calculate_discount;
DELIMITER $$
CREATE FUNCTION calculate_discount(price DECIMAL(10,2), discount_pct INT)
RETURNS DECIMAL(10,2)
DETERMINISTIC
BEGIN
    RETURN ROUND(price * (1 - discount_pct / 100), 2);
END$$
DELIMITER ;

-- Function 2: Calculate Days Since Order
DROP FUNCTION IF EXISTS days_since_order;
DELIMITER $$
CREATE FUNCTION days_since_order(order_date DATETIME)
RETURNS INT
DETERMINISTIC
BEGIN
    RETURN DATEDIFF(NOW(), order_date);
END$$
DELIMITER ;

-- Function 3: Get Customer Status
DROP FUNCTION IF EXISTS get_customer_status;
DELIMITER $$
CREATE FUNCTION get_customer_status(cust_id INT)
RETURNS VARCHAR(20)
READS SQL DATA
BEGIN
    DECLARE cust_status VARCHAR(20);
    SELECT status INTO cust_status 
    FROM customers 
    WHERE customer_id = cust_id;
    RETURN IFNULL(cust_status, 'unknown');
END$$
DELIMITER ;

-- Function 4: Calculate Order Total
DROP FUNCTION IF EXISTS calculate_order_total;
DELIMITER $$
CREATE FUNCTION calculate_order_total(ord_id INT)
RETURNS DECIMAL(10,2)
READS SQL DATA
BEGIN
    DECLARE total DECIMAL(10,2);
    SELECT SUM(quantity * unit_price) INTO total
    FROM order_items
    WHERE order_id = ord_id;
    RETURN IFNULL(total, 0.00);
END$$
DELIMITER ;

-- Function 5: Check Stock Availability
DROP FUNCTION IF EXISTS check_stock_availability;
DELIMITER $$
CREATE FUNCTION check_stock_availability(prod_id INT, requested_qty INT)
RETURNS BOOLEAN
READS SQL DATA
BEGIN
    DECLARE available_stock INT;
    SELECT stock_quantity INTO available_stock
    FROM products
    WHERE product_id = prod_id;
    
    IF available_stock >= requested_qty THEN
        RETURN TRUE;
    ELSE
        RETURN FALSE;
    END IF;
END$$
DELIMITER ;

-- ====================================================================
-- 4. CREATE STORED PROCEDURES
-- ====================================================================

-- Procedure 1: Update Customer Status
DROP PROCEDURE IF EXISTS update_customer_status;
DELIMITER $$
CREATE PROCEDURE update_customer_status(
    IN cust_id INT, 
    IN new_status VARCHAR(20)
)
BEGIN
    UPDATE customers 
    SET status = new_status, 
        updated_at = NOW()
    WHERE customer_id = cust_id;
END$$
DELIMITER ;

-- Procedure 2: Add Order
DROP PROCEDURE IF EXISTS add_order;
DELIMITER $$
CREATE PROCEDURE add_order(
    IN cust_id INT,
    IN total DECIMAL(10,2),
    OUT new_order_id INT
)
BEGIN
    INSERT INTO orders (customer_id, order_date, total_amount, status)
    VALUES (cust_id, NOW(), total, 'pending');
    
    SET new_order_id = LAST_INSERT_ID();
END$$
DELIMITER ;

-- Procedure 3: Update Product Stock
DROP PROCEDURE IF EXISTS update_product_stock;
DELIMITER $$
CREATE PROCEDURE update_product_stock(
    IN prod_id INT,
    IN qty_change INT
)
BEGIN
    UPDATE products
    SET stock_quantity = stock_quantity + qty_change,
        updated_at = NOW()
    WHERE product_id = prod_id;
END$$
DELIMITER ;

-- Procedure 4: Get Customer Orders
DROP PROCEDURE IF EXISTS get_customer_orders;
DELIMITER $$
CREATE PROCEDURE get_customer_orders(IN cust_id INT)
BEGIN
    SELECT 
        o.order_id,
        o.order_date,
        o.total_amount,
        o.status,
        COUNT(oi.item_id) as item_count
    FROM orders o
    LEFT JOIN order_items oi ON o.order_id = oi.order_id
    WHERE o.customer_id = cust_id
    GROUP BY o.order_id, o.order_date, o.total_amount, o.status
    ORDER BY o.order_date DESC;
END$$
DELIMITER ;

-- Procedure 5: Calculate Customer Lifetime Value
DROP PROCEDURE IF EXISTS calculate_customer_ltv;
DELIMITER $$
CREATE PROCEDURE calculate_customer_ltv(
    IN cust_id INT,
    OUT lifetime_value DECIMAL(10,2)
)
BEGIN
    SELECT IFNULL(SUM(total_amount), 0.00) INTO lifetime_value
    FROM orders
    WHERE customer_id = cust_id;
    
    UPDATE customers
    SET total_spent = lifetime_value
    WHERE customer_id = cust_id;
END$$
DELIMITER ;

-- ====================================================================
-- 5. CREATE TRIGGERS
-- ====================================================================

-- Trigger 1: Before Insert on Orders - Set timestamps
DROP TRIGGER IF EXISTS before_order_insert;
DELIMITER $$
CREATE TRIGGER before_order_insert
BEFORE INSERT ON orders
FOR EACH ROW
BEGIN
    IF NEW.order_date IS NULL THEN
        SET NEW.order_date = NOW();
    END IF;
    SET NEW.created_at = NOW();
    SET NEW.updated_at = NOW();
END$$
DELIMITER ;

-- Trigger 2: After Insert on Orders - Log to audit table
DROP TRIGGER IF EXISTS after_order_insert;
DELIMITER $$
CREATE TRIGGER after_order_insert
AFTER INSERT ON orders
FOR EACH ROW
BEGIN
    INSERT INTO audit_log (table_name, action, new_value, changed_at)
    VALUES ('orders', 'INSERT', 
            CONCAT('Order ID: ', NEW.order_id, ', Customer: ', NEW.customer_id, ', Amount: ', NEW.total_amount),
            NOW());
END$$
DELIMITER ;

-- Trigger 3: Before Update on Products - Validate stock
DROP TRIGGER IF EXISTS before_product_update;
DELIMITER $$
CREATE TRIGGER before_product_update
BEFORE UPDATE ON products
FOR EACH ROW
BEGIN
    IF NEW.stock_quantity < 0 THEN
        SIGNAL SQLSTATE '45000'
        SET MESSAGE_TEXT = 'Stock quantity cannot be negative';
    END IF;
    SET NEW.updated_at = NOW();
END$$
DELIMITER ;

-- Trigger 4: After Update on Products - Log price changes
DROP TRIGGER IF EXISTS after_product_update;
DELIMITER $$
CREATE TRIGGER after_product_update
AFTER UPDATE ON products
FOR EACH ROW
BEGIN
    IF OLD.price != NEW.price THEN
        INSERT INTO audit_log (table_name, action, old_value, new_value, changed_at)
        VALUES ('products', 'UPDATE',
                CONCAT('Product: ', OLD.name, ', Old Price: ', OLD.price),
                CONCAT('Product: ', NEW.name, ', New Price: ', NEW.price),
                NOW());
    END IF;
END$$
DELIMITER ;

-- Trigger 5: After Delete on Customers - Log deletion
DROP TRIGGER IF EXISTS after_customer_delete;
DELIMITER $$
CREATE TRIGGER after_customer_delete
AFTER DELETE ON customers
FOR EACH ROW
BEGIN
    INSERT INTO audit_log (table_name, action, old_value, changed_at)
    VALUES ('customers', 'DELETE',
            CONCAT('Customer: ', OLD.name, ', Email: ', OLD.email),
            NOW());
END$$
DELIMITER ;

-- Trigger 6: Before Update on Customers - Auto-update timestamp
DROP TRIGGER IF EXISTS before_customer_update;
DELIMITER $$
CREATE TRIGGER before_customer_update
BEFORE UPDATE ON customers
FOR EACH ROW
BEGIN
    SET NEW.updated_at = NOW();
END$$
DELIMITER ;

-- ====================================================================
-- 6. VERIFICATION QUERIES
-- ====================================================================

-- Show all views
SELECT TABLE_NAME, TABLE_SCHEMA 
FROM INFORMATION_SCHEMA.VIEWS 
WHERE TABLE_SCHEMA = DATABASE()
ORDER BY TABLE_NAME;

-- Show all functions
SELECT ROUTINE_NAME, ROUTINE_TYPE, DTD_IDENTIFIER as RETURNS
FROM INFORMATION_SCHEMA.ROUTINES
WHERE ROUTINE_SCHEMA = DATABASE()
AND ROUTINE_TYPE = 'FUNCTION'
ORDER BY ROUTINE_NAME;

-- Show all procedures
SELECT ROUTINE_NAME, ROUTINE_TYPE
FROM INFORMATION_SCHEMA.ROUTINES
WHERE ROUTINE_SCHEMA = DATABASE()
AND ROUTINE_TYPE = 'PROCEDURE'
ORDER BY ROUTINE_NAME;

-- Show all triggers
SELECT TRIGGER_NAME, EVENT_OBJECT_TABLE, ACTION_TIMING, EVENT_MANIPULATION
FROM INFORMATION_SCHEMA.TRIGGERS
WHERE TRIGGER_SCHEMA = DATABASE()
ORDER BY EVENT_OBJECT_TABLE, TRIGGER_NAME;

-- ====================================================================
-- 7. TEST THE OBJECTS
-- ====================================================================

-- Test Views
SELECT 'Testing customer_summary view:' as test;
SELECT * FROM customer_summary LIMIT 5;

SELECT 'Testing active_customers view:' as test;
SELECT * FROM active_customers LIMIT 5;

SELECT 'Testing product_inventory view:' as test;
SELECT * FROM product_inventory LIMIT 5;

-- Test Functions
SELECT 'Testing calculate_discount function:' as test;
SELECT calculate_discount(100.00, 10) as discounted_price;

SELECT 'Testing days_since_order function:' as test;
SELECT days_since_order('2024-01-01 10:00:00') as days_ago;

SELECT 'Testing get_customer_status function:' as test;
SELECT get_customer_status(1) as customer_status;

-- Test Procedures
SELECT 'Testing update_customer_status procedure:' as test;
CALL update_customer_status(1, 'active');

SELECT 'Testing add_order procedure:' as test;
CALL add_order(1, 150.00, @new_id);
SELECT @new_id as new_order_id;

-- Test Triggers (they fire automatically on INSERT/UPDATE/DELETE)
SELECT 'Testing triggers by inserting an order:' as test;
INSERT INTO orders (customer_id, total_amount, status) 
VALUES (1, 99.99, 'pending');

-- Check audit log to see trigger results
SELECT 'Checking audit log (trigger results):' as test;
SELECT * FROM audit_log ORDER BY changed_at DESC LIMIT 5;

-- ====================================================================
-- SUMMARY
-- ====================================================================

SELECT '====================================================================';
SELECT 'Database Objects Created Successfully!' as status;
SELECT '====================================================================';

SELECT 'Summary:' as '';
SELECT CONCAT(COUNT(*), ' views created') as summary
FROM INFORMATION_SCHEMA.VIEWS 
WHERE TABLE_SCHEMA = DATABASE();

SELECT CONCAT(COUNT(*), ' functions created') as summary
FROM INFORMATION_SCHEMA.ROUTINES
WHERE ROUTINE_SCHEMA = DATABASE()
AND ROUTINE_TYPE = 'FUNCTION';

SELECT CONCAT(COUNT(*), ' procedures created') as summary
FROM INFORMATION_SCHEMA.ROUTINES
WHERE ROUTINE_SCHEMA = DATABASE()
AND ROUTINE_TYPE = 'PROCEDURE';

SELECT CONCAT(COUNT(*), ' triggers created') as summary
FROM INFORMATION_SCHEMA.TRIGGERS
WHERE TRIGGER_SCHEMA = DATABASE();

SELECT '====================================================================';
SELECT 'Now run the migration with: ./rebuild-and-run.sh --full-sync' as next_step;
SELECT '====================================================================';

