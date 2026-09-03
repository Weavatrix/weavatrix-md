import express from "express";
import { Kafka } from "kafkajs";

const app = express();
app.post("/charge", () => undefined);

const producer = new Kafka({ brokers: ["kafka.internal:9092"] }).producer();
await producer.send({ topic: "payment.completed", messages: [] });
