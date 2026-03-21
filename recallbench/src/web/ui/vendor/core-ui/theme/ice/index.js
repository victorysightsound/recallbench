import ice from './object.js';
import { addPrefix } from '../../functions/addPrefix.js';

export default ({ addBase, prefix = '' }) => {
  const prefixedice = addPrefix(ice, prefix);
  addBase({ ...prefixedice });
};
