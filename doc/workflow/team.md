# Team workflow

## Individual roles and responsibilities


| Role | Responsibility | Assignee |
|------|----------------|----------|
| Reviewer | Ensure code consistency and that no error are present in the code of an other team member | Everyone |
| DevOps/Infra Engineer | Sets up and maintain CI/CD pipeline as well as dev and prod infrastructure | Aubry |
| Release Manager | Handles releases, responsible for validating PRs of the `release` branch | - |
| PO, PM | Sprint coordinator, manages the product backlog, assign tasks to team members at the beginning of every sprint, responsible of the daily message to the client | - |
| Documentation/QA Lead | Keeps API documentation up to date, maintains README, manages integration testing | - |


Every team member is responsible for unit testing on its own features before delivering them

Every team member is responsible for documenting its own features

TODO: Once architecture is designed, add responsibilities of every part of the architecture

Even if any team member could work on any part of the application, every member is responsible for a part of the architecture

## Communication

### Channels
Keeping good and efficient communication between team members is key to consistency, therefore two main communication channels have been adopted. 

First the Whatsapp group for fast questions, comments or other that need fast answers (less than an hour). 

Then for questions or remarks that are worth for documenting the project timeline (architecture, conception, etc.) add comments to your issues and ping people you want help from. On the other hand don't forget to frequently verify your Github notifications so you don't miss them


### Submitting a PR
Depending on the type of PRs (feature addition, bugfix, documentation update) the corresponding template should be used.


(Could be added in the `Contributing` section of the README)

Actual templates lie in `.github/pull_request_template/` and can be used directly when opening a PR by adding the corresponding query parameter to the URL:
- `?template=feature.md` for feature addition
- `?template=bugfix.md` for bugfixes
- `?template=documentation.md` for documentation updates

TODO: Create issue to add those files in the right dir 

## Agile methodology

Based on the relatively low time assigned to this project, sprints of 3 days have been chosen.

### Sprint events

1. Sprint planning

At the beginning of every sprint, a meeting is appointed to define sprint objectives, select features to implement, define the acceptance criteria and fragment the backlog elements in smaller chunks of work that should take less than a day

2. Daily Scrum

15 minutes meetings to discuss progress and issues, output should be an action plan for the day

Two daily scrums must be attended, first one at 09:30AM aiming at planning the day and discuss previous day issues. Second one at 01:30PM to adjust daily work

3. Sprint review

At the end of every sprint, useful to plan next activities, discuss actual status of the dev. etc.



### Project timeline visualization

We use a Kanban to visualize work that needs to be done for the sprint in progress

4 columns:
- Backlog: Contains the whole backlog of the sprint
- Doing: Tasks in progress
- Review: Waiting for review
- Done: PR has been closed, tests passed


