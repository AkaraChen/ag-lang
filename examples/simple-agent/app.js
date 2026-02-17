import { PromptTemplate } from "@agentscript/prompt-runtime";
const system_prompt = new PromptTemplate({
    messages: [
        {
            role: "system",
            content: "You are a helpful coding assistant.\nYou specialize in TypeScript and Rust programming.\nAlways be concise and accurate."
        }
    ]
});
import { PromptTemplate } from "@agentscript/prompt-runtime";
const greeting = new PromptTemplate({
    messages: [
        {
            role: "user",
            content: "Hello, what can you help me with today?"
        }
    ]
});
function lookup_docs(topic) {
    return `Documentation for: ${topic}`;
}
function calculate(op, a, b) {
    return (()=>{
        const _match = op;
        if (_match === "add") {
            return a + b;
        } else if (_match === "sub") {
            return a - b;
        } else if (_match === "mul") {
            return a * b;
        } else if (_match === "div") {
            return b === 0 ? 0 : a / b;
        } else {
            return 0;
        }
    })();
}
function format_result(value) {
    return (()=>{
        const _match = value;
        if (_match === null) {
            return "(no result)";
        } else {
            return `Result: ${value}`;
        }
    })();
}
function classify_question(q) {
    return q === "math" ? "calculation" : q === "docs" ? "documentation" : "general";
}
function process(input) {
    return format_result(classify_question(input));
}
export async function run(question) {
    const category = classify_question(question);
    const answer = (()=>{
        const _match = category;
        if (_match === "calculation") {
            return format_result(calculate("add", 2, 3));
        } else if (_match === "documentation") {
            return lookup_docs(question);
        } else {
            return "I can help with that!";
        }
    })();
    return answer;
}
