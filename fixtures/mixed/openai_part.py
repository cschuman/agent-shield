import openai

client = openai.OpenAI()
assistant = client.beta.assistants.create(
    name="Code Reviewer",
    instructions="Review code for security issues.",
    model="gpt-4-turbo",
)
