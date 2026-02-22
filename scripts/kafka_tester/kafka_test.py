#!/usr/bin/env python3
"""
Kafka Topic Consumer using confluent-kafka
"""

from confluent_kafka import Consumer, KafkaError
import json
import signal
import sys
import time

# Configuration with better settings
conf = {
    'bootstrap.servers': 'localhost:9092',
    'group.id': 'topic_viewer_group',
    'auto.offset.reset': 'earliest',
    'enable.auto.commit': True,
    'session.timeout.ms': 6000,
    'max.poll.interval.ms': 300000,
    'broker.address.family': 'v4',  # Force IPv4
    'socket.timeout.ms': 10000,      # Longer timeout for slow connections
    'debug': 'all'                    # Enable debug to see what's happening
}

# Topics to consume
topics = ['playback.sessions', 'user.reactions', 'user.profiles', 'notifications']

# Create consumer
consumer = Consumer(conf)
consumer.subscribe(topics)

print(f"✅ Connecting to Kafka at localhost:9092")
print(f"📋 Subscribed to topics: {', '.join(topics)}")
print("Press Ctrl+C to exit\n")

# Wait for connection
time.sleep(2)

def signal_handler(sig, frame):
    print("\n\n🛑 Shutting down...")
    consumer.close()
    sys.exit(0)

signal.signal(signal.SIGINT, signal_handler)

try:
    while True:
        msg = consumer.poll(1.0)  # Timeout in seconds
        
        if msg is None:
            continue
        if msg.error():
            if msg.error().code() == KafkaError._PARTITION_EOF:
                # End of partition, not a real error
                continue
            else:
                print(f"Error: {msg.error()}")
                continue
        
        # Print message details
        print(f"\n{'='*80}")
        print(f"📌 Topic: {msg.topic()}")
        print(f"📌 Partition: {msg.partition()}")
        print(f"📌 Offset: {msg.offset()}")
        
        if msg.key():
            print(f"🔑 Key: {msg.key().decode('utf-8')}")
        
        # Decode and parse value
        try:
            value = json.loads(msg.value().decode('utf-8'))
            print(f"📦 Value:")
            print(json.dumps(value, indent=2, default=str))
        except json.JSONDecodeError:
            # If not JSON, print as string
            print(f"📦 Value: {msg.value().decode('utf-8')}")
        except Exception as e:
            print(f"📦 Value: {msg.value()}")
        
        print(f"{'='*80}")

except Exception as e:
    print(f"Error: {e}")
finally:
    consumer.close()
