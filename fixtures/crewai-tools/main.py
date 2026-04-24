"""CrewAI multi-agent fixture without human-in-the-loop."""
from crewai import Crew, Agent, Task

researcher = Agent(
    name="Researcher",
    role="Senior Research Analyst",
    goal="Discover relevant information",
)

writer = Agent(
    name="Writer",
    role="Content Strategist",
    goal="Synthesize findings into a draft",
)

research_task = Task(description="Research the topic", agent=researcher)
writing_task = Task(description="Write the draft", agent=writer)

crew = Crew(
    agents=[researcher, writer],
    tasks=[research_task, writing_task],
)
