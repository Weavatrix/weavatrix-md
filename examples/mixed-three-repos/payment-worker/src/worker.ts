import { Kafka } from "kafkajs";

const consumer = new Kafka({ brokers: ["kafka.internal:9092"] }).consumer({
  groupId: "payment-worker",
});
await consumer.subscribe({ topic: "payment.completed" });
