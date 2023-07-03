activity-tracker-title = Activity Tracker

activity-tracker-play-time = {$hours ->
    [0] {$minutes ->
            [one] 1 minute
            *[other] {$minutes} minutes
        }
    [one] 1 hour and {$minutes ->
            [one] 1 minute
            *[other] {$minutes} minutes
        }
   *[other] {$hours} hours and {$minutes ->
            [one] 1 minute
            *[other] {$minutes} minutes
        }
}