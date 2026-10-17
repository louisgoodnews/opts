# Lighting quality (graphics page section, the automatic mode and the Environment page row).

set-light-section = Lighting quality
set-light-section-note = Light shafts, light cones, mirrors, reflections and bloom. In automatic mode OPTS lowers these in steps when the frame rate drops and raises them again when there is room.
set-light-auto-note = Automatic, level { $level } ({ $name }): the controls below show what is applied right now. Choose custom to set them yourself.
set-light-auto-disabled = Automatic mode decides this. Switch the lighting quality mode to custom to change it.
set-light-level-0 = full
set-light-level-1 = fewer cones
set-light-level-2 = headlights only
set-light-level-3 = cones off
set-light-level-4 = sun rays only
set-light-level-5 = volumetrics off

set-light-mode = Lighting quality mode
set-light-mode-desc = Automatic: OPTS picks the lighting level from the measured frame time. Custom: you choose every effect yourself and nothing changes by itself. Switching to custom keeps the level reached as the starting values; switching back to automatic starts again at the fullest level.
set-opt-lightmode-automatic = Automatic
set-opt-lightmode-custom = Custom

set-light-shafts = Sun rays
set-light-shafts-desc = Light shafts through the air (volumetric fog). The quality is the number of steps per ray: low 8, medium 16, high 32, ultra 64. Off draws none and costs nothing.
set-opt-shafts-off = Off
set-opt-shafts-low = Low (8)
set-opt-shafts-medium = Medium (16)
set-opt-shafts-high = High (32)
set-opt-shafts-ultra = Ultra (64)

set-light-night-shafts = Rays at night
set-light-night-shafts-desc = Also draws light shafts at night, lit by the moon. Costs more at night because of the many lamps.
set-light-lantern-cones = Cones of street lamps
set-light-lantern-cones-desc = Visible cones of light below street lamps and other lanterns.
set-light-headlight-cones = Cones of headlights
set-light-headlight-cones-desc = Visible cones of light in front of vehicle headlights.
set-light-cone-cap = Most light cones
set-light-cone-cap-desc = How many light cones are drawn at most at the same time, from 0 to 96. The nearest ones are kept.
set-light-cone-depth-fade = Soft cone edges
set-light-cone-depth-fade-desc = Cones fade out where they meet walls and the ground instead of cutting through them. Needs an extra depth pass and costs frame time.
set-mirror-count = Mirrors
set-mirror-count-desc = How many mirrors show a live rear view, 0 to 2. Each one renders the scene again.
set-mirror-rate = Mirror update rate
set-mirror-rate-desc = How often a mirror renders: every frame, every 2nd or every 4th frame. Less often is cheaper.
set-opt-mirror-rate-1 = Every frame
set-opt-mirror-rate-2 = Every 2nd frame
set-opt-mirror-rate-4 = Every 4th frame
set-glass-reflections = Sky in glass
set-glass-reflections-desc = Windows and other glass reflect the sky.
set-bloom = Bloom
set-bloom-desc = A soft glow around very bright lights.

env-light-quality = Lighting quality
env-light-automatic = Automatic (level { $level }: { $name })
env-light-custom = Custom
