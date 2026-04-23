import pytest

pytestmark = pytest.mark.asyncio(loop_scope="session")


class TestSoundCRUD:
    """Test sound CRUD operations."""

    async def test_add_and_retrieve_sounds(self, database):
        await database.add_sound(1, 'filenameA', 'text A', 'tags A')
        await database.add_sound(2, 'filenameB', 'text B', 'tags B')
        await database.add_sound(3, 'filenameC', 'text C', 'tags C')
        await database.add_sound(4, 'filenameD', 'text D', 'tags D')
        assert len(await database.get_sounds()) >= 4

    async def test_retrieve_by_filename(self, database):
        assert await database.get_sound(filename='filenameA') is not None

    async def test_retrieve_by_id(self, database):
        assert await database.get_sound(id=2) is not None

    async def test_retrieve_by_both(self, database):
        assert await database.get_sound(id=3, filename='filenameC') is not None

    async def test_retrieve_mismatched_returns_none(self, database):
        assert await database.get_sound(id=4, filename='filenameB') is None

    async def test_delete_sound_without_uses(self, database):
        """Sounds without usage history should be hard-deleted."""
        initial_count = len(await database.get_sounds())
        sound = await database.get_sound(id=2)
        await database.delete_sound(sound)
        assert len(await database.get_sounds()) == initial_count - 1


class TestUserCRUD:
    """Test user CRUD operations."""

    async def test_add_user(self, database):
        user = {
            'id': 1,
            'is_bot': False,
            'first_name': 'first name',
            'username': 'username',
            'last_name': None,
            'language_code': 'en-US'
        }
        await database.add_or_update_user(user)
        assert await database.get_user(username='username') == user

    async def test_add_user_with_nulls(self, database):
        user = {
            'id': 2,
            'is_bot': True,
            'first_name': 'first name',
            'username': None,
            'last_name': None,
            'language_code': None
        }
        await database.add_or_update_user(user)
        assert await database.get_user(id=2) == user

    async def test_retrieve_by_id_and_username(self, database):
        user = {
            'id': 1,
            'is_bot': False,
            'first_name': 'first name',
            'username': 'username',
            'last_name': None,
            'language_code': 'en-US'
        }
        assert await database.get_user(id=1, username='username') == user

    async def test_nonexistent_user_returns_none(self, database):
        assert await database.get_user(id=999) is None

    async def test_update_user(self, database):
        updated = {
            'id': 1,
            'is_bot': False,
            'first_name': 'first name',
            'username': 'new_username',
            'last_name': None,
            'language_code': 'en-US'
        }
        await database.add_or_update_user(updated)
        db_user = await database.get_user(id=1)
        assert db_user == updated
