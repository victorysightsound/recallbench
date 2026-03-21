import dusk from './object.js';
import { addPrefix } from '../../functions/addPrefix.js';

export default ({ addBase, prefix = '' }) => {
  const prefixeddusk = addPrefix(dusk, prefix);
  addBase({ ...prefixeddusk });
};
