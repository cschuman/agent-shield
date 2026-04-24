"""Minimal LangChain agent fixture."""
import langchain
from langchain.agents import AgentExecutor, create_react_agent
from langchain_openai import ChatOpenAI

llm = ChatOpenAI(model="gpt-4")

agent = create_react_agent(llm=llm, tools=[], prompt=None)
executor = AgentExecutor(agent=agent, tools=[])
