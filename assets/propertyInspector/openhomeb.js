'use strict';

const ACTIONS = {
  'com.infamous-pattern.openhomeb.devices': { kind: 'devices', title: 'OpenHomeB Devices' },
  'com.infamous-pattern.openhomeb.switch': { kind: 'switch', title: 'Switch' },
  'com.infamous-pattern.openhomeb.brightness': { kind: 'brightness', title: 'Brightness' },
  'com.infamous-pattern.openhomeb.set': { kind: 'set', title: 'Set State' },
  'com.infamous-pattern.openhomeb.adjust': { kind: 'adjust', title: 'Adjust State' },
  'com.infamous-pattern.openhomeb.config-ui': { kind: 'openUi', title: 'Open Homebridge UI' }
};

let websocket = null;
let pluginUUID = null;
let actionUUID = null;
let contextUUID = null;
let actionKind = 'devices';
let globalSettings = {};
let actionSettings = {};
let catalog = null;
let actionSaveTimer = null;
let connectionEdited = false;
const elements = {};

window.addEventListener('DOMContentLoaded', () => {
  for (const id of [
    'actionTitle', 'homebridgeUrl', 'username', 'password', 'updateInterval',
    'catalogCacheSeconds', 'otp', 'connectButton', 'refreshButton', 'selectionPanel',
    'roomSelect', 'deviceTypeSelect', 'serviceSelect', 'characteristicLabel',
    'characteristicSelect', 'displayName', 'labelPanel', 'labelMode', 'showConfirmation',
    'switchTestPanel', 'testSwitchButton', 'brightnessPanel', 'brightnessMode',
    'brightnessIncrement', 'brightnessTargetLabel', 'brightnessTarget',
    'brightnessCycleLabel', 'brightnessCycleValues', 'brightnessWrap',
    'brightnessTurnOn', 'testBrightnessButton', 'targetPanel', 'targetInputLabel',
    'targetInput', 'targetSelectLabel', 'targetSelect', 'valuePreview', 'speedPanel',
    'speed', 'resultsTitle', 'statusBadge', 'summary', 'error', 'diagnostics',
    'authMethod', 'tokenRefresh', 'catalogStatus', 'catalogRefreshed', 'deviceList'
  ]) elements[id] = document.getElementById(id);

  for (const id of ['homebridgeUrl', 'username', 'password', 'updateInterval', 'catalogCacheSeconds']) {
    elements[id].addEventListener('input', markConnectionEdited);
  }

  for (const id of [
    'displayName', 'targetInput', 'speed', 'brightnessIncrement',
    'brightnessTarget', 'brightnessCycleValues'
  ]) {
    elements[id].addEventListener('input', scheduleActionSave);
  }

  for (const id of [
    'targetSelect', 'labelMode', 'showConfirmation', 'brightnessMode',
    'brightnessWrap', 'brightnessTurnOn'
  ]) {
    elements[id].addEventListener('change', () => {
      if (id === 'brightnessMode') configureBrightnessControls();
      saveActionSettings();
    });
  }

  elements.roomSelect.addEventListener('change', renderServiceOptions);
  elements.deviceTypeSelect.addEventListener('change', renderServiceOptions);
  elements.serviceSelect.addEventListener('change', onServiceChanged);
  elements.characteristicSelect.addEventListener('change', onCharacteristicChanged);
  elements.connectButton.addEventListener('click', () => requestCatalog(false));
  elements.refreshButton.addEventListener('click', () => requestCatalog(true));
  elements.testSwitchButton.addEventListener('click', requestSwitchToggle);
  elements.testBrightnessButton.addEventListener('click', requestBrightnessTest);
});

function connectElgatoStreamDeckSocket(inPort, inPluginUUID, inRegisterEvent, inInfo, inActionInfo) {
  pluginUUID = inPluginUUID;
  const info = JSON.parse(inActionInfo);
  actionUUID = info.action;
  contextUUID = info.context;
  actionSettings = info.payload.settings || {};
  actionKind = (ACTIONS[actionUUID] || ACTIONS['com.infamous-pattern.openhomeb.devices']).kind;

  websocket = new WebSocket(`ws://localhost:${inPort}`);
  websocket.onopen = () => {
    websocket.send(JSON.stringify({ event: inRegisterEvent, uuid: pluginUUID }));
    websocket.send(JSON.stringify({ event: 'getGlobalSettings', context: pluginUUID }));
    configureForAction();
    populateActionFields();
  };
  websocket.onmessage = event => handleMessage(JSON.parse(event.data));
  websocket.onerror = () => setStatus('error', 'OpenDeck connection error');
}

function configureForAction() {
  const meta = ACTIONS[actionUUID] || { title: 'OpenHomeB', kind: actionKind };
  elements.actionTitle.textContent = meta.title;
  const needsSelection = ['switch', 'brightness', 'set', 'adjust'].includes(actionKind);
  elements.selectionPanel.hidden = !needsSelection;
  elements.switchTestPanel.hidden = actionKind !== 'switch';
  elements.brightnessPanel.hidden = actionKind !== 'brightness';
  elements.targetPanel.hidden = actionKind !== 'set';
  elements.speedPanel.hidden = actionKind !== 'adjust';
  elements.labelPanel.hidden = !['switch', 'brightness'].includes(actionKind);
  elements.characteristicLabel.hidden = actionKind === 'brightness';
  elements.resultsTitle.textContent = actionKind === 'devices' ? 'Discovered devices' : 'Connection status';
  configureLabelOptions();
  configureBrightnessControls();
}

function configureLabelOptions() {
  const allowed = actionKind === 'brightness'
    ? new Set(['nameAndValue', 'valueOnly', 'nameOnly', 'hidden'])
    : new Set(['nameAndState', 'stateOnly', 'nameOnly', 'hidden']);
  for (const option of elements.labelMode.options) option.hidden = !allowed.has(option.value);
}

function handleMessage(message) {
  if (message.event === 'didReceiveGlobalSettings') {
    globalSettings = message.payload?.settings || {};
    if (!connectionEdited) populateGlobalFields();
    return;
  }
  if (message.event === 'didReceiveSettings' && message.payload?.settings) {
    actionSettings = message.payload.settings;
    populateActionFields();
    return;
  }
  if (message.event !== 'sendToPropertyInspector' || !message.payload) return;
  const payload = message.payload;
  if (payload.event === 'status') {
    setStatus(payload.status, payload.message);
    if (payload.status === 'error') {
      elements.error.textContent = payload.message;
      elements.error.hidden = false;
    }
    elements.connectButton.disabled = false;
    elements.refreshButton.disabled = false;
    return;
  }
  if (payload.event === 'catalog') {
    catalog = payload.catalog;
    elements.error.hidden = true;
    elements.connectButton.disabled = false;
    elements.refreshButton.disabled = false;
    elements.connectButton.textContent = 'Save and connect';
    elements.otp.value = '';
    connectionEdited = false;
    renderCatalog();
  }
}

function populateGlobalFields() {
  elements.homebridgeUrl.value = globalSettings.homebridgeUrl || 'http://homebridge.local:8581';
  elements.username.value = globalSettings.username || '';
  elements.password.value = globalSettings.password || '';
  elements.updateInterval.value = Number(globalSettings.updateInterval || 5);
  elements.catalogCacheSeconds.value = Number(globalSettings.catalogCacheSeconds || 60);
}

function populateActionFields() {
  elements.displayName.value = actionSettings.displayName || '';
  elements.speed.value = Number(actionSettings.speed || 1);
  elements.labelMode.value = actionSettings.labelMode || (actionKind === 'brightness' ? 'nameAndValue' : 'nameAndState');
  elements.showConfirmation.checked = actionSettings.showConfirmation !== false;
  elements.brightnessMode.value = actionSettings.mode || 'increase';
  elements.brightnessIncrement.value = Number(actionSettings.increment || 10);
  elements.brightnessTarget.value = Number(actionSettings.targetValue ?? 50);
  elements.brightnessCycleValues.value = Array.isArray(actionSettings.cycleValues)
    ? actionSettings.cycleValues.join(', ')
    : '25, 50, 75, 100';
  elements.brightnessWrap.checked = actionSettings.wrap !== false;
  elements.brightnessTurnOn.checked = actionSettings.turnOnWhenAdjusting !== false;
  configureLabelOptions();
  configureBrightnessControls();
  if (catalog) renderCatalog();
}

function markConnectionEdited() {
  connectionEdited = true;
  elements.connectButton.textContent = 'Save and connect';
  if (elements.statusBadge.classList.contains('connected')) {
    setStatus('idle', 'Connection changes are not saved until you select Save and connect.');
  }
}

function normaliseHomebridgeUrl(input) {
  let value = String(input || '').trim().replace(/\/+$/, '');
  if (!value) throw new Error('Enter the Homebridge UI address, for example http://homebridge.local:8581.');
  if (/^[a-z][a-z0-9+.-]*:\/\//i.test(value) && !/^https?:\/\//i.test(value)) {
    throw new Error('The Homebridge address must use http:// or https://.');
  }
  if (!/^https?:\/\//i.test(value)) value = `http://${value}`;

  let parsed;
  try {
    parsed = new URL(value);
  } catch {
    throw new Error('The Homebridge address or port is invalid. Example: http://10.52.10.19:8581.');
  }
  if (!['http:', 'https:'].includes(parsed.protocol)) throw new Error('The Homebridge address must use http:// or https://.');
  if (!parsed.hostname) throw new Error('The Homebridge address must include a host name or IP address.');
  if (parsed.username || parsed.password) throw new Error('Do not put credentials in the URL; use the username and password fields.');
  if (parsed.search || parsed.hash) throw new Error('The Homebridge address cannot include a query string or fragment.');

  let path = parsed.pathname.replace(/\/+$/, '');
  if (path.endsWith('/api')) path = path.slice(0, -4);
  parsed.pathname = path || '/';
  parsed.search = '';
  parsed.hash = '';
  return parsed.toString().replace(/\/$/, '');
}

function collectGlobalSettings() {
  const interval = Number(elements.updateInterval.value || 5);
  const cacheSeconds = Number(elements.catalogCacheSeconds.value || 60);
  if (!Number.isFinite(interval) || interval < 1 || interval > 3600) {
    throw new Error('State refresh must be between 1 and 3600 seconds.');
  }
  if (!Number.isFinite(cacheSeconds) || cacheSeconds < 5 || cacheSeconds > 3600) {
    throw new Error('Device cache must be between 5 and 3600 seconds.');
  }
  return {
    homebridgeUrl: normaliseHomebridgeUrl(elements.homebridgeUrl.value),
    username: elements.username.value.trim(),
    password: elements.password.value,
    updateInterval: Math.round(interval),
    catalogCacheSeconds: Math.round(cacheSeconds)
  };
}

function characteristicTypeOf(characteristic) {
  return String(characteristic?.characteristicType || characteristic?.type || '').trim();
}

function collectActionSettings() {
  const useSelectors = Boolean(catalog && ['switch', 'brightness', 'set', 'adjust'].includes(actionKind));
  const characteristic = useSelectors ? selectedCharacteristic() : null;
  const accessoryId = useSelectors ? elements.serviceSelect.value : (actionSettings.accessoryId || '');
  const characteristicType = characteristic
    ? characteristicTypeOf(characteristic)
    : (useSelectors ? '' : (actionSettings.characteristicType || ''));
  const characteristicUuid = characteristic?.uuid || (useSelectors ? '' : (actionSettings.characteristicUuid || ''));

  if (actionKind === 'switch') {
    return {
      accessoryId,
      characteristicType,
      characteristicUuid,
      displayName: elements.displayName.value.trim(),
      labelMode: elements.labelMode.value || 'nameAndState',
      showConfirmation: elements.showConfirmation.checked
    };
  }
  if (actionKind === 'brightness') {
    return {
      accessoryId,
      characteristicType: characteristicType || 'Brightness',
      characteristicUuid,
      displayName: elements.displayName.value.trim(),
      mode: elements.brightnessMode.value || 'increase',
      increment: positiveNumber(elements.brightnessIncrement.value, 10),
      targetValue: finiteNumber(elements.brightnessTarget.value, 50),
      cycleValues: parseCycleValues(elements.brightnessCycleValues.value),
      wrap: elements.brightnessWrap.checked,
      labelMode: elements.labelMode.value || 'nameAndValue',
      turnOnWhenAdjusting: elements.brightnessTurnOn.checked,
      showConfirmation: elements.showConfirmation.checked
    };
  }
  if (actionKind === 'set') {
    return {
      accessoryId,
      characteristicType,
      characteristicUuid,
      targetValue: readTargetValue(),
      displayName: elements.displayName.value.trim()
    };
  }
  if (actionKind === 'adjust') {
    return {
      accessoryId,
      characteristicType,
      characteristicUuid,
      speed: positiveNumber(elements.speed.value, 1),
      displayName: elements.displayName.value.trim()
    };
  }
  return {};
}

function finiteNumber(value, fallback) {
  const number = Number(value);
  return Number.isFinite(number) ? number : fallback;
}

function positiveNumber(value, fallback) {
  const number = finiteNumber(value, fallback);
  return number > 0 ? number : fallback;
}

function parseCycleValues(value) {
  const values = String(value || '')
    .split(/[;,\s]+/)
    .map(item => item.trim())
    .filter(Boolean)
    .map(Number)
    .filter(Number.isFinite);
  return values.length ? [...new Set(values)].sort((a, b) => a - b) : [25, 50, 75, 100];
}

function scheduleActionSave() {
  clearTimeout(actionSaveTimer);
  actionSaveTimer = setTimeout(saveActionSettings, 300);
}

function saveActionSettings() {
  if (!websocket || websocket.readyState !== WebSocket.OPEN) return;
  actionSettings = collectActionSettings();
  websocket.send(JSON.stringify({
    event: 'setSettings',
    context: contextUUID,
    payload: actionSettings
  }));
}

function requestCatalog(forceRefresh) {
  if (!websocket || websocket.readyState !== WebSocket.OPEN) return;

  let connection;
  try {
    connection = collectGlobalSettings();
  } catch (error) {
    setStatus('error', error.message);
    elements.error.textContent = error.message;
    elements.error.hidden = false;
    return;
  }

  globalSettings = connection;
  actionSettings = collectActionSettings();
  setStatus('connecting', forceRefresh ? 'Refreshing devices from Homebridge…' : 'Saving settings and connecting…');
  elements.error.hidden = true;
  elements.connectButton.disabled = true;
  elements.refreshButton.disabled = true;
  websocket.send(JSON.stringify({
    event: 'sendToPlugin',
    action: actionUUID,
    context: contextUUID,
    payload: {
      event: 'refreshCatalog',
      globalSettings: connection,
      actionSettings,
      otp: elements.otp.value.trim(),
      forceRefresh: Boolean(forceRefresh)
    }
  }));
}

function requestSwitchToggle() {
  sendActionTest('toggleSwitch', 'Select a Homebridge service and writable Boolean characteristic first.', 'Sending switch command to Homebridge…');
}

function requestBrightnessTest() {
  sendActionTest('testBrightness', 'Select a service with a writable Brightness characteristic first.', 'Sending brightness command to Homebridge…');
}

function sendActionTest(eventName, missingMessage, progressMessage) {
  if (!websocket || websocket.readyState !== WebSocket.OPEN) return;
  const settings = collectActionSettings();
  if (!settings.accessoryId || (!settings.characteristicType && !settings.characteristicUuid)) {
    setStatus('error', missingMessage);
    return;
  }
  actionSettings = settings;
  websocket.send(JSON.stringify({ event: 'setSettings', context: contextUUID, payload: actionSettings }));
  setStatus('connecting', progressMessage);
  websocket.send(JSON.stringify({
    event: 'sendToPlugin',
    action: actionUUID,
    context: contextUUID,
    payload: { event: eventName, actionSettings }
  }));
}

function setStatus(status, message) {
  elements.statusBadge.className = `badge ${status}`;
  elements.statusBadge.textContent = status === 'connecting'
    ? 'Connecting'
    : status === 'error'
      ? 'Error'
      : status === 'warning'
        ? 'Cached'
        : status === 'connected'
          ? 'Connected'
          : 'Not connected';
  elements.summary.textContent = message;
}

function renderCatalog() {
  if (!catalog) return;
  const reconciliation = reconcileActionSelection();
  const deviceWord = catalog.deviceCount === 1 ? 'device' : 'devices';
  const serviceWord = catalog.serviceCount === 1 ? 'service' : 'services';
  const cacheText = catalog.stale
    ? 'Showing stale cached data.'
    : catalog.cached
      ? `Shared cache: ${catalog.cacheAgeSeconds || 0}s old.`
      : 'Live catalogue.';
  const baseSummary = `${catalog.deviceCount} ${deviceWord}, ${catalog.serviceCount} ${serviceWord}. ${cacheText}`;
  setStatus(catalog.stale ? 'warning' : 'connected', reconciliation.message ? `${baseSummary} ${reconciliation.message}` : baseSummary);
  renderDiagnostics();
  if (actionKind === 'devices') renderDeviceCards();
  if (['switch', 'brightness', 'set', 'adjust'].includes(actionKind)) renderSelectors();
  if (actionKind === 'openUi') {
    elements.deviceList.replaceChildren();
    const message = document.createElement('p');
    message.className = 'empty';
    message.textContent = 'Press the OpenDeck key to open this Homebridge UI in your Linux default browser.';
    elements.deviceList.appendChild(message);
  }
  if (reconciliation.changed) saveActionSettings();
}

function renderDiagnostics() {
  const auth = catalog.authenticationStatus || {};
  elements.diagnostics.hidden = false;
  elements.authMethod.textContent = auth.method || catalog.authentication || 'Unknown';
  elements.tokenRefresh.textContent = auth.refreshAtEpochMs
    ? `${formatDate(auth.refreshAtEpochMs)} (${formatDuration(auth.remainingSeconds || 0)} remaining)`
    : 'Not reported';
  elements.catalogStatus.textContent = catalog.stale
    ? `Stale cache · ${catalog.warning || 'live refresh unavailable'}`
    : catalog.cached
      ? `Shared cache · ${catalog.cacheAgeSeconds || 0} seconds old`
      : 'Live response';
  elements.catalogRefreshed.textContent = catalog.refreshedAtEpochMs ? formatDate(catalog.refreshedAtEpochMs) : 'Unknown';
}

function formatDate(epochMs) {
  const date = new Date(Number(epochMs));
  return Number.isNaN(date.getTime()) ? 'Unknown' : date.toLocaleString();
}

function formatDuration(seconds) {
  const total = Math.max(0, Number(seconds) || 0);
  const days = Math.floor(total / 86400);
  const hours = Math.floor((total % 86400) / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  if (days) return `${days}d ${hours}h`;
  if (hours) return `${hours}h ${minutes}m`;
  return `${minutes}m`;
}

function serviceAllowed(item) {
  const characteristics = item.service.serviceCharacteristics || [];
  if (actionKind === 'switch') return characteristics.some(isWritableBooleanCharacteristic);
  if (actionKind === 'brightness') return characteristics.some(isBrightnessCharacteristic);
  if (actionKind === 'adjust') return characteristics.some(characteristicAllowed);
  if (actionKind === 'set') return characteristics.some(characteristic => characteristic.canWrite);
  return true;
}

function deviceTypeAllowed(item) {
  const filter = elements.deviceTypeSelect.value;
  if (!filter) return true;
  const type = `${item.service.serviceType || ''} ${item.service.humanType || ''} ${item.service.type || ''}`.toLowerCase();
  const characteristicTypes = (item.service.serviceCharacteristics || []).map(characteristic => characteristicTypeOf(characteristic).toLowerCase());
  if (filter === 'light') return type.includes('light') || characteristicTypes.includes('brightness');
  if (filter === 'switch') return type.includes('switch') || type.includes('outlet');
  if (filter === 'fan') return type.includes('fan');
  if (filter === 'covering') return type.includes('window') || type.includes('door') || type.includes('garage');
  if (filter === 'thermostat') return type.includes('thermostat') || type.includes('heater') || type.includes('cooler');
  if (filter === 'sensor') return type.includes('sensor') || type.includes('detector');
  return true;
}

function findCharacteristicBySettings(service) {
  const characteristics = service.serviceCharacteristics || [];
  const uuid = String(actionSettings.characteristicUuid || '').trim().toLowerCase();
  if (uuid) {
    const byUuid = characteristics.find(item => String(item.uuid || '').trim().toLowerCase() === uuid);
    if (byUuid) return byUuid;
  }
  const type = String(actionSettings.characteristicType || '').trim().toLowerCase();
  const byType = characteristics.find(item => characteristicTypeOf(item).toLowerCase() === type);
  if (byType) return byType;
  if (actionKind === 'brightness') return characteristics.find(isBrightnessCharacteristic) || null;
  return null;
}

function reconcileActionSelection() {
  if (!['switch', 'brightness', 'set', 'adjust'].includes(actionKind)) return { changed: false, message: '' };

  let changed = false;
  let message = '';
  const selectedId = String(actionSettings.accessoryId || '');
  const item = (catalog.services || []).find(candidate => candidate.service.uniqueId === selectedId && serviceAllowed(candidate));

  if (selectedId && !item) {
    actionSettings.accessoryId = '';
    actionSettings.characteristicType = '';
    actionSettings.characteristicUuid = '';
    changed = true;
    message = 'The previous service selection is no longer available and was cleared.';
    return { changed, message };
  }
  if (!item) return { changed, message };

  const hadCharacteristic = Boolean(actionSettings.characteristicType || actionSettings.characteristicUuid);
  const characteristic = findCharacteristicBySettings(item.service);
  if (hadCharacteristic && (!characteristic || !characteristicAllowed(characteristic))) {
    actionSettings.characteristicType = '';
    actionSettings.characteristicUuid = '';
    changed = true;
    message = 'The previous characteristic selection is no longer available and was cleared.';
    return { changed, message };
  }

  if (characteristic) {
    const canonicalType = characteristicTypeOf(characteristic);
    const canonicalUuid = characteristic.uuid || '';
    if (actionSettings.characteristicType !== canonicalType || actionSettings.characteristicUuid !== canonicalUuid) {
      actionSettings.characteristicType = canonicalType;
      actionSettings.characteristicUuid = canonicalUuid;
      changed = true;
    }
  }
  return { changed, message };
}

function renderSelectors() {
  const previousRoom = elements.roomSelect.value;
  elements.roomSelect.replaceChildren(new Option('All rooms', ''));
  for (const room of catalog.rooms || []) elements.roomSelect.add(new Option(room, room));
  if ([...elements.roomSelect.options].some(option => option.value === previousRoom)) elements.roomSelect.value = previousRoom;
  renderServiceOptions();
}

function filteredServices() {
  const room = elements.roomSelect.value;
  return (catalog.services || []).filter(item => {
    if (room && item.roomName !== room) return false;
    return serviceAllowed(item) && deviceTypeAllowed(item);
  });
}

function renderServiceOptions() {
  const selected = actionSettings.accessoryId || elements.serviceSelect.value;
  elements.serviceSelect.replaceChildren(new Option('Select a Homebridge service', ''));
  for (const item of filteredServices()) {
    const service = item.service;
    const metadata = item.deviceMetadata || {};
    const hardware = [metadata.manufacturer, metadata.model].filter(Boolean).join(' ');
    const label = [
      item.roomName,
      item.customName || service.serviceName || service.accessoryName,
      service.serviceType,
      hardware
    ].filter(Boolean).join(' — ');
    elements.serviceSelect.add(new Option(label, service.uniqueId));
  }
  if ([...elements.serviceSelect.options].some(option => option.value === selected)) elements.serviceSelect.value = selected;
  renderCharacteristicOptions();
}

function selectedCatalogService() {
  const id = elements.serviceSelect.value;
  return (catalog?.services || []).find(item => item.service.uniqueId === id) || null;
}

function onServiceChanged() {
  const item = selectedCatalogService();
  if (item && !elements.displayName.value.trim()) {
    elements.displayName.value = item.customName || item.service.serviceName || item.service.accessoryName || '';
  }
  actionSettings.accessoryId = elements.serviceSelect.value;
  actionSettings.characteristicType = '';
  actionSettings.characteristicUuid = '';
  renderCharacteristicOptions();
  saveActionSettings();
}

function isWritableBooleanCharacteristic(characteristic) {
  const format = String(characteristic.format || '').toLowerCase();
  const isBoolean = format === 'bool' || typeof characteristic.value === 'boolean';
  return Boolean(characteristic.canRead && characteristic.canWrite && isBoolean);
}

function isBrightnessCharacteristic(characteristic) {
  const type = characteristicTypeOf(characteristic).toLowerCase();
  const description = String(characteristic.description || '').toLowerCase();
  const format = String(characteristic.format || '').toLowerCase();
  const numeric = ['int', 'float', 'uint8', 'uint16', 'uint32', 'uint64'].includes(format) || typeof characteristic.value === 'number';
  return Boolean(characteristic.canRead && characteristic.canWrite && numeric && (type === 'brightness' || description === 'brightness'));
}

function characteristicAllowed(characteristic) {
  if (actionKind === 'switch') return isWritableBooleanCharacteristic(characteristic);
  if (actionKind === 'brightness') return isBrightnessCharacteristic(characteristic);
  if (!characteristic.canWrite) return false;
  if (actionKind === 'adjust') {
    const format = String(characteristic.format || '').toLowerCase();
    return ['int', 'float', 'uint8', 'uint16', 'uint32', 'uint64'].includes(format) || typeof characteristic.value === 'number';
  }
  return true;
}

function characteristicKey(characteristic) {
  if (characteristic.uuid) return `uuid:${String(characteristic.uuid).toLowerCase()}`;
  return `type:${characteristicTypeOf(characteristic).toLowerCase()}`;
}

function renderCharacteristicOptions() {
  const item = selectedCatalogService();
  const configured = item ? findCharacteristicBySettings(item.service) : null;
  const selected = configured ? characteristicKey(configured) : elements.characteristicSelect.value;
  elements.characteristicSelect.replaceChildren(new Option('Select a characteristic', ''));
  if (!item) {
    configureTargetEditor(null);
    return;
  }
  for (const characteristic of item.service.serviceCharacteristics || []) {
    if (!characteristicAllowed(characteristic)) continue;
    const description = characteristic.description || characteristicTypeOf(characteristic) || 'Characteristic';
    const current = actionKind === 'switch'
      ? (readBooleanValue(characteristic.value) ? 'On' : 'Off')
      : formatValue(characteristic.value);
    elements.characteristicSelect.add(new Option(`${description} — Current: ${current}`, characteristicKey(characteristic)));
  }
  if ([...elements.characteristicSelect.options].some(option => option.value === selected)) {
    elements.characteristicSelect.value = selected;
  } else if (elements.characteristicSelect.options.length === 2) {
    elements.characteristicSelect.selectedIndex = 1;
    const characteristic = selectedCharacteristic();
    actionSettings.characteristicType = characteristic ? characteristicTypeOf(characteristic) : '';
    actionSettings.characteristicUuid = characteristic?.uuid || '';
    scheduleActionSave();
  }
  configureTargetEditor(selectedCharacteristic());
}

function readBooleanValue(value) {
  if (typeof value === 'boolean') return value;
  if (value === 1 || String(value).trim().toLowerCase() === 'true' || String(value).trim().toLowerCase() === 'on') return true;
  return false;
}

function selectedCharacteristic() {
  const item = selectedCatalogService();
  if (!item) return null;
  const key = elements.characteristicSelect.value;
  return (item.service.serviceCharacteristics || []).find(characteristic => characteristicKey(characteristic) === key) || null;
}

function onCharacteristicChanged() {
  const characteristic = selectedCharacteristic();
  actionSettings.characteristicType = characteristic ? characteristicTypeOf(characteristic) : '';
  actionSettings.characteristicUuid = characteristic?.uuid || '';
  configureTargetEditor(characteristic);
  saveActionSettings();
}

function configureBrightnessControls() {
  const mode = elements.brightnessMode?.value || actionSettings.mode || 'increase';
  if (!elements.brightnessTargetLabel) return;
  elements.brightnessTargetLabel.hidden = mode !== 'set';
  elements.brightnessCycleLabel.hidden = mode !== 'cycle';
  elements.brightnessWrap.closest('label').hidden = mode !== 'cycle';
}

function configureTargetEditor(characteristic) {
  if (actionKind !== 'set') return;
  if (!characteristic) {
    elements.targetInputLabel.hidden = false;
    elements.targetSelectLabel.hidden = true;
    elements.targetInput.value = '';
    elements.valuePreview.textContent = 'Select a writable characteristic.';
    return;
  }

  const currentTarget = actionSettings.targetValue ?? characteristic.value;
  const format = String(characteristic.format || '').toLowerCase();
  const validValues = characteristic.validValues || [];
  const isBoolean = format === 'bool' || typeof characteristic.value === 'boolean';

  if (isBoolean || validValues.length) {
    elements.targetInputLabel.hidden = true;
    elements.targetSelectLabel.hidden = false;
    elements.targetSelect.replaceChildren();
    const choices = isBoolean ? [false, true] : validValues;
    for (const value of choices) elements.targetSelect.add(new Option(formatValue(value), JSON.stringify(value)));
    const encodedCurrent = JSON.stringify(currentTarget);
    if ([...elements.targetSelect.options].some(option => option.value === encodedCurrent)) elements.targetSelect.value = encodedCurrent;
  } else {
    elements.targetInputLabel.hidden = false;
    elements.targetSelectLabel.hidden = true;
    const numeric = ['int', 'float', 'uint8', 'uint16', 'uint32', 'uint64'].includes(format) || typeof characteristic.value === 'number';
    elements.targetInput.type = numeric ? 'number' : 'text';
    elements.targetInput.min = characteristic.minValue ?? '';
    elements.targetInput.max = characteristic.maxValue ?? '';
    elements.targetInput.step = characteristic.minStep ?? (format === 'float' ? '0.1' : '1');
    elements.targetInput.value = currentTarget ?? '';
  }
  elements.valuePreview.textContent = `Current: ${formatValue(characteristic.value)} · Format: ${characteristic.format || 'unknown'}`;
}

function readTargetValue() {
  const characteristic = selectedCharacteristic();
  if (!characteristic) return actionSettings.targetValue ?? null;
  if (!elements.targetSelectLabel.hidden) {
    try { return JSON.parse(elements.targetSelect.value); } catch { return elements.targetSelect.value; }
  }
  const format = String(characteristic.format || '').toLowerCase();
  const numeric = ['int', 'float', 'uint8', 'uint16', 'uint32', 'uint64'].includes(format) || typeof characteristic.value === 'number';
  return numeric ? Number(elements.targetInput.value) : elements.targetInput.value;
}

function renderDeviceCards() {
  elements.deviceList.replaceChildren();
  const grouped = new Map();
  for (const item of catalog.services || []) {
    const service = item.service;
    const bridgeName = service.instance?.name || 'Homebridge';
    const key = `${bridgeName}\u001f${service.accessoryName}`;
    if (!grouped.has(key)) grouped.set(key, {
      bridgeName,
      accessoryName: service.accessoryName,
      metadata: item.deviceMetadata || {},
      services: []
    });
    grouped.get(key).services.push(item);
  }
  if (!grouped.size) {
    const empty = document.createElement('p');
    empty.className = 'empty';
    empty.textContent = 'Homebridge returned no accessories.';
    elements.deviceList.appendChild(empty);
    return;
  }
  for (const device of grouped.values()) {
    const card = document.createElement('article');
    card.className = 'device-card';
    const heading = document.createElement('h3');
    heading.textContent = device.accessoryName;
    card.appendChild(heading);
    const meta = document.createElement('p');
    meta.className = 'meta';
    const hardware = [device.metadata.manufacturer, device.metadata.model, device.metadata.serialNumber].filter(Boolean).join(' · ');
    meta.textContent = [device.bridgeName, hardware].filter(Boolean).join(' · ');
    card.appendChild(meta);
    const list = document.createElement('ul');
    for (const item of device.services) {
      const service = item.service;
      const row = document.createElement('li');
      const name = document.createElement('strong');
      name.textContent = item.customName || service.serviceName;
      row.appendChild(name);
      const detail = document.createElement('span');
      detail.textContent = `${item.roomName} · ${service.serviceType} · ${(service.serviceCharacteristics || []).length} characteristics`;
      row.appendChild(detail);
      const id = document.createElement('code');
      id.textContent = service.uniqueId;
      row.appendChild(id);
      list.appendChild(row);
    }
    card.appendChild(list);
    elements.deviceList.appendChild(card);
  }
}

function formatValue(value) {
  if (value === null || value === undefined) return 'Unavailable';
  if (typeof value === 'boolean') return value ? 'On' : 'Off';
  if (typeof value === 'number' && !Number.isInteger(value)) return String(Math.round(value * 100) / 100);
  return String(value);
}
