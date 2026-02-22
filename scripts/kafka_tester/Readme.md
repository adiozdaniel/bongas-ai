# Test Kafka Connection

## Installation and Usage

### Create a virtual environment

```bash
python3 -m venv venv
```

### Activate the virtual environment

```bash
source venv/bin/activate
```

### **Install required package:**

```bash
pip install -r requirements.txt
```

### **Run the enhanced consumer:**

```bash
# Default usage (localhost:9092)
python kafka_test.py

# Custom bootstrap servers
python kafka_test.py --bootstrap-servers kafka1:9092,kafka2:9092

# Custom consumer group
python kafka_test.py --group-id my_group

# Consume specific topics
python kafka_test.py --topics playback.sessions user.reactions
```

### **Run the simple consumer:**

```bash
python simple_consumer.py
```

## Features

The enhanced consumer includes:

- **Formatted output** with emojis and clear section separation
- **Topic-specific formatting** for each message type
- **Graceful shutdown** with Ctrl+C
- **Error handling** for connection issues
- **Configurable** bootstrap servers and consumer group
- **JSON deserialization** with fallback to string/bytes
- **Timestamp display** in readable format

The script will continue running until you press Ctrl+C, displaying messages as they arrive on the Kafka topics.
