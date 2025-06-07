use thiserror::Error; // 引入 thiserror 库的 Error trait，用于方便地定义错误类型

#[derive(Error, Debug)] // 派生 Error 和 Debug trait
pub enum BlockchainError { // 定义区块链错误枚举类型
    #[error("Connection error: {0}")] // 连接错误，包含一个描述错误的字符串参数
    ConnectionError(String), // 连接错误变体
    
    #[error("Invalid address")] // 无效地址错误
    InvalidAddress, // 无效地址变体
    
    #[error("Account not found")] // 账户未找到错误
    AccountNotFound, // 账户未找到变体
    
    #[error("Storage error: {0}")] // 存储错误，包含一个描述错误的字符串参数
    StorageError(String), // 存储错误变体
    
    #[error("Decode error: {0}")] // 解码错误，包含一个描述错误的字符串参数
    DecodeError(String), // 解码错误变体

    #[error("Transaction error: {0}")] // 交易错误，包含一个描述错误的字符串参数
    TransactionError(String), // 交易错误变体

    #[error("Query error: {0}")] // 查询错误，包含一个描述错误的字符串参数
    QueryError(String), // 查询错误变体
}