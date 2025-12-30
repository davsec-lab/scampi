#include "neo4j.h"
#include <curl/curl.h>
#include <jsoncpp/json/json.h>
#include <iostream>

size_t Neo4jClient::WriteCallback(void* contents, size_t size, size_t nmemb, HTTPResponse* response) {
    size_t totalSize = size * nmemb;
    response->data.append((char*)contents, totalSize);
    return totalSize;
}

Neo4jClient::Neo4jClient(const std::string& url, const std::string& username, const std::string& password) 
    : baseUrl(url) {
    auth = username + ":" + password;
    
    // Initialize curl globally (should be done once per application)
    curl_global_init(CURL_GLOBAL_DEFAULT);
}

Neo4jClient::~Neo4jClient() {
    // Flush any remaining queries before cleanup
    flushQueries();
    
    // Cleanup curl globally (should be done once per application)
    curl_global_cleanup();
}

std::string Neo4jClient::executeQuery(const std::string& cypher) {
    CURL* curl = curl_easy_init();
    HTTPResponse response;
    
    if (curl) {
        // Prepare JSON payload using jsoncpp
        Json::Value root;
        Json::Value statements(Json::arrayValue);
        Json::Value statement;
        statement["statement"] = cypher;
        statements.append(statement);
        root["statements"] = statements;
        
        Json::StreamWriterBuilder builder;
        std::string jsonString = Json::writeString(builder, root);
        
        // Set headers
        struct curl_slist* headers = NULL;
        headers = curl_slist_append(headers, "Content-Type: application/json");
        headers = curl_slist_append(headers, "Accept: application/json");
        
        // Configure curl
        std::string url = baseUrl + "/db/neo4j/tx/commit";
        curl_easy_setopt(curl, CURLOPT_URL, url.c_str());
        curl_easy_setopt(curl, CURLOPT_POSTFIELDS, jsonString.c_str());
        curl_easy_setopt(curl, CURLOPT_HTTPHEADER, headers);
        curl_easy_setopt(curl, CURLOPT_USERPWD, auth.c_str());
        curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, WriteCallback);
        curl_easy_setopt(curl, CURLOPT_WRITEDATA, &response);
        
        // Execute request
        CURLcode res = curl_easy_perform(curl);
        if (res != CURLE_OK) {
            std::cerr << "curl_easy_perform() failed: " << curl_easy_strerror(res) << std::endl;
        }
        
        // Get HTTP response code
        long httpCode = 0;
        curl_easy_getinfo(curl, CURLINFO_RESPONSE_CODE, &httpCode);
        
        if (httpCode != 200 && httpCode != 201) {
            std::cerr << "HTTP Error: " << httpCode << std::endl;
        }
        
        curl_slist_free_all(headers);
        curl_easy_cleanup(curl);
    }
    
    return response.data;
}

void Neo4jClient::submitQuery(const std::string& cypher) {
    queryQueue.push(cypher);
    
    // If we've reached the batch size, execute all queries
    if (queryQueue.size() >= BATCH_SIZE) {
        flushQueries();
    }
}

void Neo4jClient::flushQueries() {
    if (queryQueue.empty()) {
        return;
    }
    
    // Collect all queries from the queue
    std::vector<std::string> queries;
    queries.reserve(queryQueue.size());
    
    while (!queryQueue.empty()) {
        queries.push_back(queryQueue.front());
        queryQueue.pop();
    }
    
    // Execute the batch
    std::string result = executeBatch(queries);
    std::cout << "Batch execution result: " << result << std::endl;
}

std::string Neo4jClient::executeBatch(const std::vector<std::string>& queries) {
    CURL* curl = curl_easy_init();
    HTTPResponse response;
    
    if (curl) {
        // Prepare JSON payload using jsoncpp
        Json::Value root;
        Json::Value statements(Json::arrayValue);
        
        for (const auto& query : queries) {
            Json::Value statement;
            statement["statement"] = query;
            statements.append(statement);
        }
        
        root["statements"] = statements;
        
        Json::StreamWriterBuilder builder;
        std::string jsonString = Json::writeString(builder, root);
        
        // Set headers
        struct curl_slist* headers = NULL;
        headers = curl_slist_append(headers, "Content-Type: application/json");
        headers = curl_slist_append(headers, "Accept: application/json");
        
        // Configure curl
        std::string url = baseUrl + "/db/neo4j/tx/commit";
        curl_easy_setopt(curl, CURLOPT_URL, url.c_str());
        curl_easy_setopt(curl, CURLOPT_POSTFIELDS, jsonString.c_str());
        curl_easy_setopt(curl, CURLOPT_HTTPHEADER, headers);
        curl_easy_setopt(curl, CURLOPT_USERPWD, auth.c_str());
        curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, WriteCallback);
        curl_easy_setopt(curl, CURLOPT_WRITEDATA, &response);
        
        // Execute request
        CURLcode res = curl_easy_perform(curl);
        if (res != CURLE_OK) {
            std::cerr << "curl_easy_perform() failed: " << curl_easy_strerror(res) << std::endl;
        }
        
        // Get HTTP response code
        long httpCode = 0;
        curl_easy_getinfo(curl, CURLINFO_RESPONSE_CODE, &httpCode);
        
        if (httpCode != 200 && httpCode != 201) {
            std::cerr << "HTTP Error: " << httpCode << std::endl;
        }
        
        curl_slist_free_all(headers);
        curl_easy_cleanup(curl);
    }
    
    return response.data;
}

std::vector<std::string> Neo4jClient::extractStringValues(const std::string& jsonResponse, const std::string& fieldName) {
    std::vector<std::string> values;
    
    try {
        Json::Value root;
        Json::Reader reader;
        
        if (!reader.parse(jsonResponse, root)) {
            std::cerr << "Failed to parse JSON response" << std::endl;
            return values;
        }
        
        // Navigate through Neo4j response structure
        if (root.isMember("results") && root["results"].isArray()) {
            for (const auto& result : root["results"]) {
                if (result.isMember("data") && result["data"].isArray()) {
                    for (const auto& dataItem : result["data"]) {
                        if (dataItem.isMember("row") && dataItem["row"].isArray()) {
                            for (const auto& rowItem : dataItem["row"]) {
                                if (rowItem.isString()) {
                                    values.push_back(rowItem.asString());
                                } else if (rowItem.isObject()) {
                                    // Handle node/relationship objects
                                    for (const auto& key : rowItem.getMemberNames()) {
                                        if (fieldName.empty() || key == fieldName) {
                                            if (rowItem[key].isString()) {
                                                values.push_back(rowItem[key].asString());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    } catch (const std::exception& e) {
        std::cerr << "Error parsing JSON: " << e.what() << std::endl;
    }
    
    return values;
}

bool Neo4jClient::testConnection() {
    std::string response = executeQuery("RETURN 1 as test");
    
    try {
        Json::Value root;
        Json::Reader reader;
        
        if (reader.parse(response, root)) {
            // Connection is good if we have results and no errors in the array
            return root.isMember("results") && 
                   root.isMember("errors") && 
                   root["errors"].isArray() && 
                   root["errors"].size() == 0;
        }
    } catch (const std::exception& e) {
        std::cerr << "Connection test failed: " << e.what() << std::endl;
    }
    
    return false;
}