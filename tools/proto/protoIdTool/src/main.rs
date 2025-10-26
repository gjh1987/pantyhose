use clap::Parser;
use std::fs;
use std::path::{Path, PathBuf};
use std::io::Write;
use anyhow::{Result, Context};
use std::collections::HashMap;

#[derive(Parser)]
#[command(name = "proto-id-tool")]
#[command(about = "Generate protobuf code with message IDs using prost")]
struct Args {
    /// Path to the proto files directory
    #[arg(long = "proto-path")]
    proto_path: String,
    
    /// Target language (only rust supported for now)
    #[arg(long = "language")]
    language: String,
    
    /// Output path for generated files
    #[arg(long = "output-path")]
    output_path: String,
    
    /// Length field size in bytes (2 for u16, 4 for u32)
    #[arg(long = "length-bytes", default_value = "2")]
    length_bytes: u8,
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    if args.language != "rust" {
        anyhow::bail!("Only rust language is supported currently");
    }
    
    if args.length_bytes != 2 && args.length_bytes != 4 {
        anyhow::bail!("length-bytes must be either 2 (u16) or 4 (u32)");
    }
    
    // 创建输出目录
    let output_dir = Path::new(&args.output_path);
    let message_dir = output_dir.join("protobuf").join("message");
    fs::create_dir_all(&message_dir)?;
    
    println!("Clearing directory: \"{}\"", message_dir.display());
    
    // 清空目录中的旧文件
    if message_dir.exists() {
        for entry in fs::read_dir(&message_dir)? {
            let entry = entry?;
            if entry.path().extension().and_then(|s| s.to_str()) == Some("rs") {
                fs::remove_file(entry.path())?;
            }
        }
    }
    
    // 查找所有 proto 文件
    let proto_files = find_proto_files(&args.proto_path)?;
    
    // 设置 protoc 路径为 vendored 版本
    std::env::set_var("PROTOC", protoc_bin_vendored::protoc_bin_path().unwrap());
    
    // 使用 prost-build 编译 proto 文件
    let mut config = prost_build::Config::new();
    
    // 设置输出目录
    config.out_dir(&message_dir);
    
    // 为所有消息类型派生额外的 trait
    config.type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]");
    
    // 编译 proto 文件
    let proto_paths: Vec<_> = proto_files.iter().map(|p| p.as_path()).collect();
    config.compile_protos(&proto_paths, &[Path::new(&args.proto_path)])?;
    
    // 解析消息以生成 message ID 和辅助代码
    let mut all_messages = Vec::new();
    for proto_file in &proto_files {
        let messages = parse_proto_file(proto_file)?;
        all_messages.extend(messages);
    }
    
    // 生成 mod.rs
    generate_mod_file(&message_dir, &proto_files)?;
    
    // 生成 protobuf_message_id.rs
    generate_message_id_file(&message_dir, &all_messages, args.length_bytes)?;
    
    println!("Found {} messages", all_messages.len());
    println!("Generated message ID file: \"{}\"", message_dir.join("protobuf_message_id.rs").display());
    
    Ok(())
}

#[derive(Debug, Clone)]
struct ProtoMessage {
    name: String,
    file_stem: String,
    message_type: MessageType,
}

#[derive(Debug, Clone, PartialEq)]
enum MessageType {
    Request,
    Response,
    Notify,
    Other,
}

fn find_proto_files(dir: &str) -> Result<Vec<PathBuf>> {
    let mut proto_files = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("proto") {
            proto_files.push(path);
        }
    }
    proto_files.sort();
    Ok(proto_files)
}

fn parse_proto_file(path: &PathBuf) -> Result<Vec<ProtoMessage>> {
    let content = fs::read_to_string(path)?;
    let file_stem = path.file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid file name"))?
        .to_string();
    
    let mut messages = Vec::new();
    let mut in_message = false;
    let mut current_message_name = String::new();
    
    for line in content.lines() {
        let trimmed = line.trim();
        
        if trimmed.starts_with("message ") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 2 {
                current_message_name = parts[1].trim_end_matches('{').to_string();
                in_message = true;
            }
        } else if in_message && trimmed == "}" {
            if !current_message_name.is_empty() && current_message_name != "MetaEntry" {
                let message_type = determine_message_type(&current_message_name);
                messages.push(ProtoMessage {
                    name: current_message_name.clone(),
                    file_stem: file_stem.clone(),
                    message_type,
                });
            }
            in_message = false;
            current_message_name.clear();
        }
    }
    
    Ok(messages)
}

fn determine_message_type(name: &str) -> MessageType {
    if name.ends_with("Request") {
        MessageType::Request
    } else if name.ends_with("Response") {
        MessageType::Response
    } else if name.ends_with("Notify") {
        MessageType::Notify
    } else {
        MessageType::Other
    }
}

fn generate_mod_file(output_dir: &Path, proto_files: &[PathBuf]) -> Result<()> {
    let mod_path = output_dir.join("mod.rs");
    let mut file = fs::File::create(&mod_path)?;
    
    writeln!(file, "// Generated by proto-id-tool")?;
    writeln!(file, "// DO NOT EDIT MANUALLY")?;
    writeln!(file)?;
    
    // 添加每个 proto 文件对应的模块
    for proto_file in proto_files {
        if let Some(stem) = proto_file.file_stem().and_then(|s| s.to_str()) {
            writeln!(file, "pub mod {};", stem)?;
        }
    }
    
    writeln!(file)?;
    writeln!(file, "pub mod protobuf_message_id;")?;
    
    println!("Generated mod.rs file: \"{}\"", mod_path.display());
    Ok(())
}

fn generate_message_id_file(output_dir: &Path, messages: &[ProtoMessage], length_bytes: u8) -> Result<()> {
    let mut content = String::new();
    
    // 文件头
    content.push_str("// Generated by proto-id-tool\n");
    content.push_str("// DO NOT EDIT MANUALLY\n\n");
    content.push_str("use crate::framework::data::DynamicBuffer;\n");
    content.push_str("use crate::proto::messages::{MessageId, MessageIdSerialize};\n");
    content.push_str("use prost::Message;\n\n");
    
    // 生成消息ID常量
    content.push_str("// Message IDs\n");
    let mut msg_id = 1u16;
    let mut message_constants = HashMap::new();
    
    for message in messages {
        let const_name = format!("MSG_ID_{}", to_snake_case(&message.name).to_uppercase());
        content.push_str(&format!("pub const {}: u16 = {};\n", const_name, msg_id));
        message_constants.insert(message.name.clone(), (const_name, msg_id));
        msg_id += 1;
    }
    
    content.push_str("\n");
    
    // 为每个消息实现 MessageId trait
    for message in messages {
        let module_name = &message.file_stem;
        let (const_name, _) = &message_constants[&message.name];
        
        content.push_str(&format!("impl MessageId for super::{}::{} {{\n", module_name, message.name));
        content.push_str(&format!("    fn msg_id(&self) -> u16 {{\n"));
        content.push_str(&format!("        {}\n", const_name));
        content.push_str(&format!("    }}\n"));
        content.push_str(&format!("}}\n\n"));
    }
    
    // 为每个消息实现 MessageIdSerialize trait
    for message in messages {
        let module_name = &message.file_stem;
        
        content.push_str(&format!("impl MessageIdSerialize for super::{}::{} {{\n", module_name, message.name));
        content.push_str("    fn serialize_to_buffer(&self) -> Result<bytes::BytesMut, Box<dyn std::error::Error + Send + Sync>> {\n");
        content.push_str("        // Get message ID\n");
        content.push_str("        let msg_id = self.msg_id();\n");
        content.push_str("        \n");
        content.push_str("        // Encode message using prost\n");
        content.push_str("        let mut data = Vec::new();\n");
        content.push_str("        self.encode(&mut data)?;\n");
        content.push_str("        let data_len = data.len();\n");
        content.push_str("        \n");
        
        if length_bytes == 2 {
            content.push_str("        // Create buffer with exact size needed (2 for msgid + 2 for length + data)\n");
            content.push_str("        let mut buffer = bytes::BytesMut::with_capacity(2 + 2 + data_len);\n");
            content.push_str("        \n");
            content.push_str("        // Write message ID (big-endian)\n");
            content.push_str("        buffer.extend_from_slice(&msg_id.to_be_bytes());\n");
            content.push_str("        \n");
            content.push_str("        // Write length as u16 (big-endian)\n");
            content.push_str("        buffer.extend_from_slice(&(data_len as u16).to_be_bytes());\n");
        } else {
            content.push_str("        // Create buffer with exact size needed (2 for msgid + 4 for length + data)\n");
            content.push_str("        let mut buffer = bytes::BytesMut::with_capacity(2 + 4 + data_len);\n");
            content.push_str("        \n");
            content.push_str("        // Write message ID (big-endian)\n");
            content.push_str("        buffer.extend_from_slice(&msg_id.to_be_bytes());\n");
            content.push_str("        \n");
            content.push_str("        // Write length as u32 (big-endian)\n");
            content.push_str("        buffer.extend_from_slice(&(data_len as u32).to_be_bytes());\n");
        }
        
        content.push_str("        \n");
        content.push_str("        // Write message data\n");
        content.push_str("        buffer.extend_from_slice(&data);\n");
        content.push_str("        \n");
        content.push_str("        Ok(buffer)\n");
        content.push_str("    }\n");
        content.push_str("}\n\n");
    }
    
    // MessageFactory
    content.push_str("/// Message decoding factory\n");
    content.push_str("pub struct MessageFactory;\n\n");
    content.push_str("impl MessageFactory {\n");
    
    // 为每个消息生成解码方法
    for message in messages {
        let module_name = &message.file_stem;
        let method_name = to_snake_case(&message.name);
        
        content.push_str(&format!("    /// Decode {} from DynamicBuffer\n", message.name));
        content.push_str(&format!("    pub fn decode_{}(buffer: &mut DynamicBuffer, length: usize) -> Option<super::{}::{}> {{\n", 
            method_name, module_name, message.name));
        content.push_str("        // Check if we have enough data\n");
        content.push_str("        if buffer.readable_bytes() < length {\n");
        content.push_str("            return None;\n");
        content.push_str("        }\n\n");
        content.push_str("        // Read exact length of data from buffer\n");
        content.push_str("        let mut data = vec![0u8; length];\n");
        content.push_str("        let bytes_read = buffer.read_bytes(&mut data, 0, length);\n");
        content.push_str("        if bytes_read != length {\n");
        content.push_str("            return None;\n");
        content.push_str("        }\n\n");
        content.push_str("        // Decode using prost\n");
        content.push_str(&format!("        super::{}::{}::decode(&data[..]).ok()\n", module_name, message.name));
        content.push_str("    }\n\n");
    }
    
    // 生成通用的 decode_message 方法
    content.push_str("    /// Decode message by ID from DynamicBuffer\n");
    content.push_str("    pub fn decode_message(msg_id: u16, buffer: &mut DynamicBuffer, length: usize) -> Option<Box<dyn std::any::Any + Send>> {\n");
    content.push_str("        match msg_id {\n");
    
    for message in messages {
        let (const_name, _) = &message_constants[&message.name];
        let method_name = to_snake_case(&message.name);
        
        content.push_str(&format!("            {} => Self::decode_{}(buffer, length).map(|m| Box::new(m) as Box<dyn std::any::Any + Send>),\n",
            const_name, method_name));
    }
    
    content.push_str("            _ => None,\n");
    content.push_str("        }\n");
    content.push_str("    }\n");
    content.push_str("}\n");
    
    // 写入文件
    let output_path = output_dir.join("protobuf_message_id.rs");
    fs::write(&output_path, content)?;
    
    Ok(())
}

fn to_snake_case(name: &str) -> String {
    let mut result = String::new();
    let mut prev_lower = false;
    let mut prev_upper = false;
    
    for (i, ch) in name.chars().enumerate() {
        if ch.is_uppercase() {
            // 在大写字母前添加下划线，但有以下例外：
            // - 如果是第一个字符
            // - 如果前一个字符也是大写字母，并且这不是连续大写字母的结尾
            if i > 0 && (prev_lower || (prev_upper && i + 1 < name.len() && name.chars().nth(i + 1).map(|c| c.is_lowercase()).unwrap_or(false))) {
                result.push('_');
            }
            result.push(ch.to_lowercase().next().unwrap());
            prev_upper = true;
            prev_lower = false;
        } else {
            result.push(ch);
            prev_upper = false;
            prev_lower = true;
        }
    }
    
    result
}