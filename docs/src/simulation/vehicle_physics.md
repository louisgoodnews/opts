# 🛞 Vehicle Physics & Kinematics Specification

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟢 Specified |
| **Author** | OPTS Architecture Team |
| **Relevant Crates** | `opts-sim`, `opts-core` |
| **Target Milestone** | [M-0022](../roadmap/phase-4/M-0022.md) |

---

## 📖 Overview

This specification establishes the deterministic physical and kinematic models governing commercial road vehicles (such as 12m standard transit buses, 15m 3-axle rigid buses, and multi-unit articulated vehicles) in `opts-sim`. The simulation operates with **zero Bevy dependencies**, utilizing classical Newtonian mechanics, deterministic fixed-step numerical integration, and geometric kinematics.

All vehicle parameter inputs are parsed by `opts-core` from OMSI vehicle manifests (`.bus`), notably [`BusManifest`](../../crates/opts-core.md) and [`BusAxle`](../../crates/opts-core.md).

---

## 🧭 Coordinate Reference Frames

Per [`coordinates.md`](../architecture/coordinates.md), vehicle simulation in `opts-sim` standardizes on the **Vehicle Local Frame**:

| Axis | Direction | Physical Meaning |
| :---: | :---: | :--- |
| **$+X$** | Right | Lateral axis (wheel track width, lateral acceleration) |
| **$+Y$** | Forward | Longitudinal axis (heading direction, tractive force) |
| **$+Z$** | Up | Vertical axis (suspension deflection, gravity $-g\hat{z}$) |

- **Handedness:** Right-handed coordinate system ($+X \times +Y = +Z$).
- **Linear Units:** Metres ($\text{m}$), Velocity in $\text{m/s}$ or $\text{km/h}$ ($1\text{ m/s} = 3.6\text{ km/h}$).
- **Angular Units:** Radians ($\text{rad}$) for computation, Degrees ($^\circ$) for user interfaces.
- **Mass Units:** Kilograms ($\text{kg}$), converted from manifest metric tons ($1\text{ t} = 1{,}000\text{ kg}$).

---

## 🛞 Multi-Axle Ackermann Steering Kinematics

Real-world commercial transit buses feature front-axle steering, with some 3-axle configurations having an active or passive rear steering tag axle.

### 1. Geometric Ackermann Equations

For a vehicle with front steering axle at longitudinal coordinate $y_{\text{steer}}$ and rear reference axle at $y_{\text{ref}}$, the effective wheelbase is:
$$L = |y_{\text{steer}} - y_{\text{ref}}|$$

When the driver requests an effective central road wheel steering angle $\delta$ around instantaneous center of rotation (ICR) with turn radius $R$:
$$R = \frac{L}{\tan(\delta)}$$

The individual wheel angles on an axle with track width $W$ are:
$$\delta_{\text{inner}} = \arctan\left(\frac{L}{R - \frac{W}{2} \cdot \operatorname{sgn}(\delta)}\right)$$
$$\delta_{\text{outer}} = \arctan\left(\frac{L}{R + \frac{W}{2} \cdot \operatorname{sgn}(\delta)}\right)$$

### 2. Zero-Angle & Straight-Ahead Defensive Guard

To prevent division-by-zero singularities when $\delta \approx 0$:
$$\text{If } |\delta| < 10^{-6}\text{ rad} \implies R \to \infty, \quad \delta_{\text{inner}} = \delta, \quad \delta_{\text{outer}} = \delta$$

### 3. Maximum Wheel Lock

Every steering axle enforces mechanical wheel cramp angles:
$$|\delta_{\text{max}}| \approx 45^\circ \text{ to } 52^\circ \quad (0.785 \text{ to } 0.907\text{ rad})$$

---

## 🏎️ Powertrain & Longitudinal Dynamics

Longitudinal motion balances tractive drive forces against motion resistances:
$$\sum F_{\text{long}} = F_{\text{traction}} - F_{\text{brake}} - F_{\text{drag}} - F_{\text{roll}} - F_{\text{grade}}$$

### 1. Engine & Standalone Powertrain Specification

When external bytecode scripts are not active, `opts-sim` utilizes a standalone parametric engine model (`EngineSpec`):
- **Idle RPM ($N_{\text{idle}}$):** Typically $600\text{ RPM}$.
- **Governor / Redline RPM ($N_{\text{max}}$):** Typically $2{,}200\text{ RPM}$.
- **Peak Torque ($T_{\text{max}}$):** Typically $1{,}100\text{ N}\cdot\text{m}$ at $1{,}200\text{ RPM}$.
- **Rated Power ($P_{\text{max}}$):** Typically $210\text{ kW}$ ($285\text{ hp}$) at $1{,}900\text{ RPM}$.

The torque curve $T_e(N)$ is calculated via a normalized polynomial torque curve matching commercial turbodiesel characteristics:
$$T_e(N) = T_{\text{max}} \cdot \max\left(0, 1.0 - 1.4 \cdot \left(\frac{N - N_{\text{peak}}}{N_{\text{max}} - N_{\text{idle}}}\right)^2\right)$$

### 2. Transmission & Drivetrain

A transmission model (`TransmissionSpec`) scales engine torque through gear ratio $i_{\text{gear}}$ and final drive differential ratio $i_{\text{diff}}$:
$$T_{\text{axle}} = T_e \cdot i_{\text{gear}} \cdot i_{\text{diff}} \cdot \eta_{\text{drivetrain}}$$
where $\eta_{\text{drivetrain}} \approx 0.88$ represents mechanical efficiency.

Standard 4-speed automatic transit bus ratios:
- **1st Gear:** $3.43$
- **2nd Gear:** $2.01$
- **3rd Gear:** $1.42$
- **4th Gear:** $1.00$
- **Reverse:** $-3.20$
- **Final Drive ($i_{\text{diff}}$):** $4.76$

Tractive force delivered at the wheels of radius $r_{\text{wheel}} = \frac{d_{\text{wheel}}}{2}$:
$$F_{\text{traction}} = \frac{T_{\text{axle}}}{r_{\text{wheel}}}$$
Distributed equally among all axles where `is_driven == true`.

---

## 🛑 Braking Systems

Commercial buses incorporate three complementary braking systems:

### 1. Service Friction Brakes

Dual-circuit pneumatic service brakes apply friction torque directly to wheel drums/discs:
$$F_{\text{service}} = \text{BrakeInput} \cdot F_{\text{brake,max}} \cdot \mu_{\text{pad}}$$
where $F_{\text{brake,max}} \approx 0.7 \cdot m \cdot g$ provides a deceleration rate of $\approx 6.8\text{ m/s}^2$ at full pedal depression.

### 2. Hydrodynamic Retarder

An auxiliary driveline brake active during deceleration:
$$T_{\text{retarder}} = \text{RetarderStage} \cdot T_{\text{retarder,max}} \cdot \min\left(1.0, \frac{v}{v_{\text{retarder,threshold}}}\right)$$
where retarder torque scales with vehicle speed down to cut-off velocity ($v \le 3\text{ km/h}$).

### 3. Spring Parking Brake (Handbrake)

When engaged ($\text{handbrake} = \text{true}$), applies 100% mechanical lock to drive axles, preventing stationary creep.

---

## 💨 Motion Resistances

### 1. Aerodynamic Drag

$$F_{\text{drag}} = \frac{1}{2} \cdot \rho_{\text{air}} \cdot C_d \cdot A_{\text{frontal}} \cdot v_{\text{long}}^2$$
- Air density $\rho_{\text{air}} = 1.225\text{ kg/m}^3$ at standard temperature/pressure.
- Drag coefficient $C_d \approx 0.65$ (bluff box body).
- Frontal area $A_{\text{frontal}} \approx 2.5\text{ m} \times 3.0\text{ m} = 7.5\text{ m}^2$.

### 2. Rolling Resistance

$$F_{\text{roll}} = C_{rr} \cdot m \cdot g$$
- Rolling resistance coefficient $C_{rr} \approx 0.012$ for heavy commercial radial tires on asphalt.

---

## 🪓 Suspension Compliance & Load Transfer

### 1. Axle Spring-Damper System

Each axle $i$ supports an equilibrium static load $W_{i,0}$ and responds to vertical deflection $\Delta z_i$ and deflection velocity $\dot{z}_i$:
$$F_{\text{susp},i} = W_{i,0} - k_i \cdot \Delta z_i - c_i \cdot \dot{z}_i$$
where $k_i$ is spring stiffness (`achse_feder`, $\approx 250\text{ kN/m}$) and $c_i$ is damping (`achse_daempfer`, $\approx 15\text{ kN}\cdot\text{s/m}$).

### 2. Dynamic Longitudinal Load Transfer

During forward acceleration $a_y$ or braking deceleration $-a_y$, inertia causes load transfer across axles based on center-of-gravity height $h_{\text{cg}}$:
$$\Delta W_{\text{long}} = m \cdot a_y \cdot \frac{h_{\text{cg}}}{L}$$
- Forward acceleration shifts vertical normal load onto the rear axle(s).
- Braking shifts vertical normal load onto the front steer axle.

---

## 🔗 Articulation Kinematics

For articulated vehicles, the tractor and trailer units are connected at hitch position:
$$\mathbf{P}_{\text{hitch}} = \mathbf{P}_{\text{tractor}} + \mathbf{R}_{\text{tractor}} \cdot \begin{bmatrix} 0 \\ y_{\text{coupling}} \\ z_{\text{coupling}} \end{bmatrix}$$

- **Hitch Angle ($\psi_{\text{art}}$):** Relative angle between tractor heading $\psi_{\text{tractor}}$ and trailer heading $\psi_{\text{trailer}}$:
  $$\psi_{\text{art}} = \psi_{\text{tractor}} - \psi_{\text{trailer}}$$
- **Mechanical Limit:** Articulation turntable stops prevent rotation past $|\psi_{\text{art}}| \le 54^\circ$ ($0.942\text{ rad}$).
- **Kinematic Following (as implemented):** The trailer front coupling point is constrained to $\mathbf{P}_{\text{hitch}}$ and its rear axle rolls without side slip, so the yaw rate is
  $$\dot{\psi}_{\text{trailer}} = \frac{\mathbf{v}_{\text{hitch}} \cdot \mathbf{r}_{\text{trailer}}}{L}$$
  with $\mathbf{v}_{\text{hitch}}$ the velocity of the hitch point (finite difference of its position), $\mathbf{r}_{\text{trailer}}$ the trailer's right-hand unit vector and $L$ the distance from the hitch to the trailer's rearmost axle (at least 1 m); see `opts_sim::vehicle::trailer` and [Articulated Vehicles](../architecture/articulation.md). The formula this page carried before, $v_{\text{tractor}}\sin(\psi_{\text{art}})/L_{\text{trailer}}$, is the special case of a hitch on the tractor's rear axle; it differs when the hitch lies behind that axle, and is not what the code does (Q-VP-01).
  The integrator step is the configured `--physics-hz` (default 64 Hz), not a fixed 60 or 120 Hz.

---

## ⏱️ Fixed-Step Numerical Integration

Vehicle physics states integrate via **semi-implicit Euler integration** at fixed time steps ($\Delta t = \frac{1}{60}\text{ s}$ or $\Delta t = \frac{1}{120}\text{ s}$):

$$\mathbf{v}(t + \Delta t) = \mathbf{v}(t) + \mathbf{a}(t) \cdot \Delta t$$
$$\mathbf{p}(t + \Delta t) = \mathbf{p}(t) + \mathbf{v}(t + \Delta t) \cdot \Delta t$$
$$\omega(t + \Delta t) = \omega(t) + \alpha(t) \cdot \Delta t$$
$$\theta(t + \Delta t) = \theta(t) + \omega(t + \Delta t) \cdot \Delta t$$

### Zero-Velocity Clamping Guard

To prevent perpetual micro-jitter when the vehicle comes to a complete standstill under braking:
$$\text{If } |\mathbf{v}| < 10^{-4}\text{ m/s and BrakeInput} > 0.05 \implies \mathbf{v} = \mathbf{0}, \quad \omega = 0$$

---

## ❓ Open Questions

| ID | Question | Status |
| :--- | :--- | :--- |
| Q-VP-01 | Is the hitch-velocity formulation of the implemented trailer follower (above) what OMSI does, or is the `sin(ψ_art)` form closer? No OMSI reference of a joint angle at a known manoeuvre exists in the repository; the design for a test is in [Articulation Physics](../architecture/articulation-physics.md). | ❓ open |
