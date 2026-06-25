//! gRPC and messaging contract detection for cross-repo linking.

use becket_schema::artifacts::SymbolRecord;

/// Role of a queue/messaging endpoint in source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueRole {
    /// Publishes or sends messages to a topic/queue.
    Producer,
    /// Subscribes or consumes messages from a topic/queue.
    Consumer,
}

/// Outbound gRPC client call site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedGrpcClient {
    /// Repository-relative file path.
    pub file_path: String,
    /// Symbol id of the calling function/method.
    pub caller_symbol_id: String,
    /// Normalized gRPC service name (e.g. `UserService`).
    pub service_name: String,
}

/// Inbound gRPC service handler registration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedGrpcServer {
    /// Repository-relative file path.
    pub file_path: String,
    /// Handler symbol id.
    pub symbol_id: String,
    /// Normalized gRPC service name.
    pub service_name: String,
}

/// Queue producer or consumer endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedQueueEndpoint {
    /// Repository-relative file path.
    pub file_path: String,
    /// Symbol id attributed to the endpoint.
    pub symbol_id: String,
    /// Topic or queue name.
    pub topic: String,
    /// Producer or consumer role.
    pub role: QueueRole,
}

/// Aggregated service-contract scan for one file.
#[derive(Debug, Clone, Default)]
pub struct ServiceContractScan {
    /// Detected gRPC client call sites.
    pub grpc_clients: Vec<ParsedGrpcClient>,
    /// Detected gRPC server registrations.
    pub grpc_servers: Vec<ParsedGrpcServer>,
    /// Detected queue producers/consumers.
    pub queue_endpoints: Vec<ParsedQueueEndpoint>,
}

/// Scans source text for gRPC and queue contract signals.
pub fn scan_service_contracts(
    file_path: &str,
    source: &str,
    symbols: &[SymbolRecord],
) -> ServiceContractScan {
    let mut scan = ServiceContractScan::default();
    for (line_idx, line) in source.lines().enumerate() {
        let line_no = line_idx + 1;
        let symbol_id = symbol_at_line(symbols, file_path, line_no);

        for service_name in detect_grpc_client_names(line) {
            scan.grpc_clients.push(ParsedGrpcClient {
                file_path: file_path.to_string(),
                caller_symbol_id: symbol_id.clone(),
                service_name,
            });
        }

        for service_name in detect_grpc_server_names(line) {
            scan.grpc_servers.push(ParsedGrpcServer {
                file_path: file_path.to_string(),
                symbol_id: symbol_id.clone(),
                service_name,
            });
        }

        for (topic, role) in detect_queue_signals(line) {
            scan.queue_endpoints.push(ParsedQueueEndpoint {
                file_path: file_path.to_string(),
                symbol_id: symbol_id.clone(),
                topic,
                role,
            });
        }
    }
    scan
}

fn symbol_at_line(symbols: &[SymbolRecord], file_path: &str, line: usize) -> String {
    let line = u32::try_from(line).unwrap_or(u32::MAX);
    symbols
        .iter()
        .filter(|symbol| symbol.file_path == file_path)
        .filter(|symbol| symbol.start_line <= line && line <= symbol.end_line)
        .max_by_key(|symbol| symbol.start_line)
        .map(|symbol| symbol.id.clone())
        .unwrap_or_else(|| format!("unknown:{file_path}:{line}"))
}

fn detect_grpc_client_names(line: &str) -> Vec<String> {
    let mut names = Vec::new();
    for token in [
        extract_regex_cap(line, r"(\w+)ServiceStub"),
        extract_regex_cap(line, r"(\w+)ServiceClient"),
        extract_regex_cap(line, r"New(\w+)Client\("),
    ]
    .into_iter()
    .flatten()
    {
        let normalized = normalize_service_name(&token);
        if !names.iter().any(|existing| existing == &normalized) {
            names.push(normalized);
        }
    }
    names
}

fn detect_grpc_server_names(line: &str) -> Vec<String> {
    let mut names = Vec::new();
    for token in [
        extract_regex_cap(line, r"add_(\w+)Servicer_to_server"),
        extract_regex_cap(line, r"Register(\w+)Server"),
        extract_regex_cap(line, r"\.register\((\w+)Service"),
        line.contains("@GrpcService")
            .then(|| extract_regex_cap(line, r"class\s+(\w+)"))
            .flatten(),
    ]
    .into_iter()
    .flatten()
    {
        let normalized = normalize_service_name(&token);
        if !names.iter().any(|existing| existing == &normalized) {
            names.push(normalized);
        }
    }
    names
}

fn detect_queue_signals(line: &str) -> Vec<(String, QueueRole)> {
    let mut signals = Vec::new();
    let producer_patterns = [
        r#"\.send\(\s*["']([^"']+)["']"#,
        r#"\.publish\(\s*["']([^"']+)["']"#,
        r#"basic_publish\([^,]*,\s*["']([^"']+)["']"#,
        r#"SendMessage\([^,]*,\s*["']([^"']+)["']"#,
    ];
    let consumer_patterns = [
        r#"\.subscribe\(\s*\[["']([^"']+)["']"#,
        r#"\.subscribe\(\s*["']([^"']+)["']"#,
        r#"@KafkaListener\([^)]*topics\s*=\s*["']([^"']+)["']"#,
        r#"@RabbitListener\([^)]*queues\s*=\s*["']([^"']+)["']"#,
        r#"ReceiveMessage\([^,]*,\s*["']([^"']+)["']"#,
    ];

    for pattern in producer_patterns {
        if let Some(topic) = extract_regex_cap(line, pattern) {
            push_unique_queue(&mut signals, topic, QueueRole::Producer);
        }
    }
    for pattern in consumer_patterns {
        if let Some(topic) = extract_regex_cap(line, pattern) {
            push_unique_queue(&mut signals, topic, QueueRole::Consumer);
        }
    }
    signals
}

fn push_unique_queue(signals: &mut Vec<(String, QueueRole)>, topic: String, role: QueueRole) {
    if signals
        .iter()
        .any(|(existing, existing_role)| existing == &topic && *existing_role == role)
    {
        return;
    }
    signals.push((topic, role));
}

fn normalize_service_name(raw: &str) -> String {
    let trimmed = raw.trim();
    let without_servicer = trimmed.strip_suffix("Servicer").unwrap_or(trimmed);
    let without_impl = without_servicer
        .strip_suffix("Impl")
        .unwrap_or(without_servicer);
    if without_impl.ends_with("Service") {
        without_impl.to_string()
    } else {
        format!("{without_impl}Service")
    }
}

fn extract_regex_cap(line: &str, pattern: &str) -> Option<String> {
    let re = regex::Regex::new(pattern).ok()?;
    let captures = re.captures(line)?;
    captures.get(1).map(|m| m.as_str().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_grpc_client_stub() {
        let scan = scan_service_contracts(
            "client.py",
            "stub = user_pb2_grpc.UserServiceStub(channel)\n",
            &[],
        );
        assert_eq!(scan.grpc_clients.len(), 1);
        assert_eq!(scan.grpc_clients[0].service_name, "UserService");
    }

    #[test]
    fn detects_queue_producer_and_consumer() {
        let scan = scan_service_contracts(
            "worker.py",
            r#"
def publish_order():
    producer.send("orders.created", value=b"x")

def consume_order():
    consumer.subscribe(["orders.created"])
"#,
            &[],
        );
        assert!(scan.queue_endpoints.iter().any(|endpoint| {
            endpoint.topic == "orders.created" && endpoint.role == QueueRole::Producer
        }));
        assert!(scan.queue_endpoints.iter().any(|endpoint| {
            endpoint.topic == "orders.created" && endpoint.role == QueueRole::Consumer
        }));
    }
}
