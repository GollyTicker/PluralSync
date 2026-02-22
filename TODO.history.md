# Text-based System Changelog

Users should be able to view a rough overview of the latest system changes in a simple text-based diff format.

The format should be easily readable for the general public, because they are not programmers and are not used to the green/red diff view.

The diff should be based on changes in the CustomFronts + Members dump of the system. (Perhaps even more? I can add that later based on user feedback.)

Whenever the existing "relevant change detection" finds a change, then the system
is marked for a full fetch of all custom fronts and members. It should do a rate limiting similar to the fronting changes rate limiting.
    wait_increment: 5s
    wait_max: 30m
    duration_to_count_over: 2h

Similar to the Fronting History, this should be configurable as well.

What about security? Currently, we save the fronting history statuses and the system changelog as plaintext. Ideally,
we should encrypt them just like we encrypt the platform secrets/tokens.

