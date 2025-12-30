#ifndef NEO4J_H
#define NEO4J_H

#include <string>
#include <vector>
#include <queue>

struct HTTPResponse {
    std::string data;
};

class Neo4jClient {
private:
    std::string baseUrl;
    std::string auth;
    std::queue<std::string> queryQueue;
    static const size_t BATCH_SIZE = 100;
    
    static size_t WriteCallback(void* contents, size_t size, size_t nmemb, HTTPResponse* response);
    std::string executeBatch(const std::vector<std::string>& queries);

public:
    Neo4jClient(const std::string& url, const std::string& username, const std::string& password);
    ~Neo4jClient();
    
    std::string executeQuery(const std::string& cypher);
    void submitQuery(const std::string& cypher);
    void flushQueries();
    std::vector<std::string> extractStringValues(const std::string& jsonResponse, const std::string& fieldName = "");
    bool testConnection();
};

#endif // NEO4J_H