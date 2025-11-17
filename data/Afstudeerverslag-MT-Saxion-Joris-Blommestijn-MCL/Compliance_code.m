% Constants
k = 7/5; % Adiabatic coefficient for air at 20°C
P0min_sys = 820; % Minimum absolute pressure for systemic chamber (mmHg)
P0max_sys = 890; % Maximum absolute pressure for systemic chamber (mmHg)
dPmax_sys = 70; % Maximum pressure difference for systemic chamber (mmHg)
dPmin_sys = 7; % Minimum absolute pressure for systemic chamber (mmHg)

Cmin_sys = 0.3; % Minimum compliance for systemic chamber (ml/mmHg)
Cmax_sys = 2.5; % Maximum compliance for systemic chamber (ml/mmHg)

P0min_pul = 760; % Minimum absolute pressure for pulmonary chamber (mmHg)
P0max_pul = 790; % Maximum absolute pressure for pulmonary chamber (mmHg)
dPmax_pul = 30; % Maximum pressure difference for pulmonary chamber (mmHg)
dPmin_pul = 4; % Minimum pressure difference for pulmonary chamber (mmHg)

Cmin_pul = 0.7; % Minimum compliance for pulmonary chamber (ml/mmHg)
Cmax_pul = 5; % Maximum compliance for pulmonary chamber (ml/mmHg)

% Calculate V0min for systemic chamber
V0min_sys = (Cmin_sys * dPmax_sys) ./ (1 - (P0min_sys ./ (P0min_sys + dPmax_sys)).^(1/k));

% Calculate V0max for systemic chamber
V0max_sys = (Cmax_sys * dPmin_sys) ./ (1 - (P0max_sys ./ (P0max_sys + dPmin_sys)).^(1/k));

% Calculate V0min for pulmonary chamber
V0min_pul = (Cmin_pul * dPmax_pul) ./ (1 - (P0min_pul ./ (P0min_pul + dPmax_pul)).^(1/k));

% Calculate V0max for pulmonary chamber
V0max_pul = (Cmax_pul * dPmin_pul) ./ (1 - (P0max_pul ./ (P0max_pul + dPmin_pul)).^(1/k));

% Display results
disp('Systemic Chamber:');
disp(['V0min = ', num2str(V0min_sys), ' mL']);
disp(['V0max = ', num2str(V0max_sys), ' mL']);
disp(' ');
disp('Pulmonary Chamber:');
disp(['V0min = ', num2str(V0min_pul), ' mL']);
disp(['V0max = ', num2str(V0max_pul), ' mL']);


%%
% MATLAB Script for Calculating Vascular Compliance

% Constants
k = 7/5; % Adiabatic coefficient for air at 20°C
Pa_to_mmHg = 133.322; % Conversion factor from Pa to mmHg

% Given physiological conditions (Table II)
systemic.Cmin = 0.3; % ml/mmHg
systemic.P0min_abs = 820; % mmHg
systemic.dPmax = 70; % mmHg
systemic.Cmax = 3; % ml/mmHg
systemic.P0max_abs = 890; % mmHg
systemic.dPmin = 7; % mmHg

pulmonary.Cmin = 0.7; % ml/mmHg
pulmonary.P0min_abs = 760; % mmHg
pulmonary.dPmax = 30; % mmHg
pulmonary.Cmax = 5; % ml/mmHg
pulmonary.P0max_abs = 790; % mmHg
pulmonary.dPmin = 4; % mmHg

% Function to calculate volume based on compliance, pressure, and dP
calculate_volume = @(C, P0, dP) (C * dP * (P0 + dP / 2)^k) / ((1 - (P0 / (P0 + dP))^(1/k)) * dP);

% Calculate V0min and V0max for systemic chamber
systemic.V0min = calculate_volume(systemic.Cmin, systemic.P0min_abs, systemic.dPmax);
systemic.V0max = calculate_volume(systemic.Cmax, systemic.P0max_abs, systemic.dPmin);

% Calculate V0min and V0max for pulmonary chamber
pulmonary.V0min = calculate_volume(pulmonary.Cmin, pulmonary.P0min_abs, pulmonary.dPmax);
pulmonary.V0max = calculate_volume(pulmonary.Cmax, pulmonary.P0max_abs, pulmonary.dPmin);

% Display results
fprintf('Systemic Chamber:\n');
fprintf('V0min = %.2f mL\n', systemic.V0min);
fprintf('V0max = %.2f mL\n\n', systemic.V0max);

fprintf('Pulmonary Chamber:\n');
fprintf('V0min = %.2f mL\n', pulmonary.V0min);
fprintf('V0max = %.2f mL\n\n', pulmonary.V0max);

% Function to adjust air volume to set desired compliance
adjust_compliance = @(V0, dP, P0, desired_C) V0 * (1 - (P0 / (P0 + dP))^(1/k)) / dP - desired_C;

% Example usage to set desired compliance
desired_C_systemic = 1.5; % ml/mmHg, desired systemic compliance
desired_C_pulmonary = 2.5; % ml/mmHg, desired pulmonary compliance

% Calculate air volume for desired compliance
V0_systemic = systemic.V0min; % Initial guess
dP_systemic = 10; % Initial pressure difference (mmHg)
P0_systemic = systemic.P0min_abs; % Initial absolute pressure (mmHg)

V0_pulmonary = pulmonary.V0min; % Initial guess
dP_pulmonary = 10; % Initial pressure difference (mmHg)
P0_pulmonary = pulmonary.P0min_abs; % Initial absolute pressure (mmHg)

% Adjusting air volume for desired compliance
adjusted_V_systemic = adjust_compliance(V0_systemic, dP_systemic, P0_systemic, desired_C_systemic);
adjusted_V_pulmonary = adjust_compliance(V0_pulmonary, dP_pulmonary, P0_pulmonary, desired_C_pulmonary);

% Display adjusted volumes for desired compliance
fprintf('Adjusted Systemic Volume for Desired Compliance:\n');
fprintf('Adjusted Volume = %.2f mL\n', adjusted_V_systemic);

fprintf('Adjusted Pulmonary Volume for Desired Compliance:\n');
fprintf('Adjusted Volume = %.2f mL\n', adjusted_V_pulmonary);



